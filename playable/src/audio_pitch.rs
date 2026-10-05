//! Original GCAX pitch helper, main.dol 8013df4c..8013e188.
//! Returns the driver's pitch scalar, not a WAV playback-rate multiplier.
//! Envelope, pitch bend, mixer and device resampling remain separate.

pub fn original_pitch(root: u8, fine: i16, note: u8) -> f32 {
    assert!(root <= 127 && note <= 127 && (-128..=255).contains(&fine));
    let mut root = i32::from(root);
    let mut fine = i32::from(fine);
    if fine < 0 {
        root += 1;
        fine += 128;
    }
    let difference = root - i32::from(note);
    let mut value = 90316.8_f64;
    let octave = if difference >= 0 { 0.5 } else { 2.0 };
    for _ in 0..difference.abs() / 12 {
        value *= octave;
    }
    for _ in 0..difference.abs() % 12 {
        if difference >= 0 {
            value /= 1.0594630943592953_f64;
        } else {
            value *= 1.0594630943592953_f64;
        }
    }
    // Original uses a strict >16 boundary, leaving 1..16 fine steps.
    if fine > 16 {
        let groups = (fine - 1) / 16;
        for _ in 0..groups {
            value *= 1.007246412223704_f64;
        }
        fine -= groups * 16;
    }
    for _ in 0..fine {
        value *= 1.0004513695322617_f64;
    }
    value as f32
}
