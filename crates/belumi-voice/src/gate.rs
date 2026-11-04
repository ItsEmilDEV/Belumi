const HANGOVER_FRAMES: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    PushToTalk,
    Activation,
}

pub fn level_db(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return -127.0;
    }
    let sum: f64 = frame
        .iter()
        .map(|s| {
            let v = f64::from(*s) / 32768.0;
            v * v
        })
        .sum();
    let rms = (sum / frame.len() as f64).sqrt();
    if rms <= 0.0 {
        -127.0
    } else {
        (20.0 * rms.log10()) as f32
    }
}

#[derive(Default)]
pub struct Gate {
    hangover: u32,
}

impl Gate {
    pub fn open(&mut self, frame: &[i16], mode: Mode, talk_key: bool, threshold_db: f32) -> bool {
        match mode {
            Mode::PushToTalk => {
                self.hangover = 0;
                talk_key
            }
            Mode::Activation => {
                if level_db(frame) >= threshold_db {
                    self.hangover = HANGOVER_FRAMES;
                    true
                } else if self.hangover > 0 {
                    self.hangover -= 1;
                    true
                } else {
                    false
                }
            }
        }
    }
}

pub fn apply_gain(frame: &mut [i16], gain: f32) {
    if (gain - 1.0).abs() < 1e-3 {
        return;
    }
    for s in frame {
        *s = (f32::from(*s) * gain).clamp(-32768.0, 32767.0) as i16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_measured_in_decibels_full_scale() {
        assert_eq!(level_db(&[0; 960]), -127.0);
        assert!((level_db(&[32767; 960]) - 0.0).abs() < 0.01);
        assert!((level_db(&[16384; 960]) + 6.02).abs() < 0.05);
        assert_eq!(level_db(&[]), -127.0);
    }

    #[test]
    fn push_to_talk_follows_the_key() {
        let mut g = Gate::default();
        assert!(g.open(&[0; 960], Mode::PushToTalk, true, -40.0));
        assert!(!g.open(&[30000; 960], Mode::PushToTalk, false, -40.0));
    }

    #[test]
    fn activation_opens_on_loud_frames_and_holds_briefly() {
        let mut g = Gate::default();
        let quiet = [10i16; 960];
        assert!(!g.open(&quiet, Mode::Activation, false, -40.0));
        assert!(g.open(&[8000; 960], Mode::Activation, false, -40.0));
        for _ in 0..HANGOVER_FRAMES {
            assert!(g.open(&quiet, Mode::Activation, false, -40.0));
        }
        assert!(!g.open(&quiet, Mode::Activation, false, -40.0));
    }

    #[test]
    fn gain_scales_and_clips() {
        let mut f = [1000i16, -1000, 30000];
        apply_gain(&mut f, 2.0);
        assert_eq!(f, [2000, -2000, 32767]);
        let mut same = [5i16];
        apply_gain(&mut same, 1.0);
        assert_eq!(same, [5]);
    }
}
