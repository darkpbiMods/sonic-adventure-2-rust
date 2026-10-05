//! Opt-in original-sample preview. Pitch/envelope settings are source-derived;
//! host resampling, 5ms envelope scheduling and volume mapping remain approximate.
use crate::{audio_envelope::Envelope, simulation::ActionEvent};
use macroquad::audio::{Sound, PlaySoundParams, load_sound_from_bytes, play_sound, stop_sound};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Deserialize)]
struct Manifest { entries: Vec<Entry>, envelope_update_seconds: f64 }
#[derive(Deserialize)]
struct Entry { name: String, file: String, sha256: String, envelope: Envelope }

pub struct ActionAudio { sounds: Vec<Sound>, pub requests: [u32; 5], pub voice_requests: u32 }
impl ActionAudio {
    pub async fn load(root: &Path) -> Result<Self, String> {
        let root = root.join("audio");
        let mut manifest: Manifest = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        if manifest.entries.len() != 5 || manifest.envelope_update_seconds != 0.005 {
            return Err("unsupported preview audio manifest".into());
        }
        let voices: Manifest = serde_json::from_slice(&std::fs::read(root.join("voice-manifest.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        if voices.envelope_update_seconds != manifest.envelope_update_seconds { return Err("unsupported voice cadence".into()); }
        manifest.entries.push(voices.entries.into_iter().find(|e|e.name=="light-voice-e").ok_or("missing English attack voice")?);
        let mut sounds = Vec::new();
        for name in ["jump", "bounce", "spin-release", "homing", "light-attack", "light-voice-e"] {
            let entry = manifest.entries.iter().find(|e|e.name == name).ok_or("missing action cue")?;
            if entry.file != format!("{name}.wav") { return Err("unexpected cue filename".into()); }
            let mut bytes = std::fs::read(root.join(&entry.file)).map_err(|e|e.to_string())?;
            if format!("{:x}", Sha256::digest(&bytes)) != entry.sha256 { return Err("action WAV hash mismatch".into()); }
            shape_wave(&mut bytes, entry.envelope.clone(), manifest.envelope_update_seconds)?;
            sounds.push(load_sound_from_bytes(&bytes).await.map_err(|e|e.to_string())?);
        }
        Ok(Self {sounds, requests: [0;5], voice_requests: 0})
    }

    pub fn event(&mut self, event: ActionEvent) {
        let index = match event {
            ActionEvent::Jump => 0,
            ActionEvent::BounceStart => 1,
            ActionEvent::SpinDash => 2,
            ActionEvent::HomingStart => 3,
            ActionEvent::LightAttackStart => 4,
            _ => return,
        };
        self.requests[index] += 1;
        play_sound(&self.sounds[index], PlaySoundParams {looped: false, volume: 0.35});
        if index==4 {
            self.voice_requests+=1;
            play_sound(&self.sounds[5], PlaySoundParams {looped: false, volume: 0.35});
        }
    }

    pub fn stop(&self) { for sound in &self.sounds { stop_sound(sound); } }
}

fn shape_wave(bytes: &mut [u8], mut envelope: Envelope, cadence: f64) -> Result<(), String> {
    // The asset preparer emits canonical mono PCM16 WAV. Reject any other layout.
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..16] != b"WAVEfmt "
        || &bytes[16..24] != [16,0,0,0,1,0,1,0] || &bytes[34..40] != [16,0,b'd',b'a',b't',b'a'] {
        return Err("unsupported action WAV format".into());
    }
    let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let length = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    if rate == 0 || length % 2 != 0 || length != bytes.len()-44 { return Err("invalid action WAV length/rate".into()); }
    let mut next_update = cadence;
    for (index, sample) in bytes[44..].chunks_exact_mut(2).enumerate() {
        let time = index as f64 / rate as f64;
        while time >= next_update { envelope.step(); next_update += cadence; }
        let gain = 10.0_f32.powf(envelope.attenuation / 20.0);
        if !gain.is_finite() { return Err("invalid action envelope gain".into()); }
        let value = (i16::from_le_bytes(sample.try_into().unwrap()) as f32 * gain).round().clamp(-32768.,32767.) as i16;
        sample.copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}
