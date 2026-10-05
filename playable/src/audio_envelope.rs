//! Original GCAX envelope step (8013c6a4..8013c890).
//! Instrument initialization and scheduling belong to the caller; this is one
//! driver update, not one gameplay frame or output sample.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Envelope {
    pub flags: u32,
    pub elapsed: f32,
    pub level: f32,
    pub attenuation: f32,
    pub threshold: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub stage: i32,
}

impl Envelope {
    fn update_attenuation(&mut self) {
        self.attenuation = 96.0_f32.mul_add(self.level, -96.0);
    }

    fn decrease(&mut self, amount: f32) {
        self.level -= amount;
        self.update_attenuation();
        if self.level <= 0.0 {
            self.level = 0.0;
            self.update_attenuation();
        }
    }

    pub fn step(&mut self) {
        if self.flags & 1 != 0 { return; }
        match self.stage {
            0 => {
                self.level += self.attack;
                self.update_attenuation();
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.update_attenuation();
                    self.stage += 1;
                }
            }
            1 => {
                if self.attenuation > self.threshold {
                    self.decrease(self.decay);
                }
                if self.attenuation <= self.threshold {
                    // Stage 2 immediately falls through to stage 3 in the source.
                    self.stage += 2;
                    self.decrease(self.sustain);
                }
            }
            2 => { self.stage += 1; self.decrease(self.sustain); }
            3 => self.decrease(self.sustain),
            4 => { self.stage += 1; self.decrease(self.release); }
            5 => self.decrease(self.release),
            _ => {}
        }
        self.elapsed += 5.0;
    }
}
