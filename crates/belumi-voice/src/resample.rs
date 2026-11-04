pub struct Resampler {
    channels: usize,
    step: f64,
    pos: f64,
    previous: Vec<f32>,
    primed: bool,
}

impl Resampler {
    pub fn new(from_rate: u32, to_rate: u32, channels: usize) -> Self {
        Self {
            channels: channels.max(1),
            step: f64::from(from_rate) / f64::from(to_rate.max(1)),
            pos: 0.0,
            previous: vec![0.0; channels.max(1)],
            primed: false,
        }
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let ch = self.channels;
        let frames = input.len() / ch;
        if frames == 0 {
            return;
        }
        if !self.primed {
            self.previous.copy_from_slice(&input[..ch]);
            self.primed = true;
        }
        let sample = |previous: &[f32], i: isize, c: usize| -> f32 {
            if i < 0 {
                previous[c]
            } else {
                input[i as usize * ch + c]
            }
        };
        let mut pos = self.pos;
        while pos < (frames - 1) as f64 {
            let i = pos.floor() as isize;
            let frac = (pos - i as f64) as f32;
            for c in 0..ch {
                let a = sample(&self.previous, i, c);
                let b = sample(&self.previous, i + 1, c);
                out.push(a + (b - a) * frac);
            }
            pos += self.step;
        }
        self.pos = pos - frames as f64;
        self.previous
            .copy_from_slice(&input[(frames - 1) * ch..frames * ch]);
    }
}

pub fn to_i16(x: f32) -> i16 {
    (x.clamp(-1.0, 1.0) * 32767.0) as i16
}

pub fn downmix(interleaved: &[f32], channels: usize, out: &mut Vec<f32>) {
    let ch = channels.max(1);
    for frame in interleaved.chunks_exact(ch) {
        out.push(frame.iter().sum::<f32>() / ch as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_rate_passes_samples_through() {
        let mut r = Resampler::new(48_000, 48_000, 1);
        let input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let mut out = Vec::new();
        r.process(&input, &mut out);
        assert_eq!(out.len(), input.len() - 1);
        for (a, b) in out.iter().zip(&input) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn lengths_follow_the_rate_ratio_across_chunks() {
        let mut r = Resampler::new(44_100, 48_000, 1);
        let mut out = Vec::new();
        for _ in 0..100 {
            r.process(&vec![0.5; 441], &mut out);
        }
        let expected = 44_100.0 * 48_000.0 / 44_100.0;
        assert!((out.len() as f64 - expected).abs() < 4.0, "{}", out.len());
        let mut down = Resampler::new(48_000, 16_000, 2);
        let mut o2 = Vec::new();
        down.process(&vec![0.1; 960], &mut o2);
        assert!((o2.len() as i64 - 320).abs() <= 2);
        assert_eq!(o2.len() % 2, 0);
    }

    #[test]
    fn a_constant_stays_constant_and_a_ramp_stays_monotonic() {
        let mut r = Resampler::new(24_000, 48_000, 1);
        let mut out = Vec::new();
        let ramp: Vec<f32> = (0..50).map(|i| i as f32).collect();
        r.process(&ramp, &mut out);
        assert!(out.windows(2).all(|w| w[1] >= w[0] - 1e-5));
        let mut r2 = Resampler::new(24_000, 48_000, 1);
        let mut c = Vec::new();
        r2.process(&[0.25; 40], &mut c);
        assert!(c.iter().all(|v| (v - 0.25).abs() < 1e-6));
    }

    #[test]
    fn downmix_averages_channels_and_conversion_clips() {
        let mut out = Vec::new();
        downmix(&[1.0, 0.0, 0.5, 0.5], 2, &mut out);
        assert_eq!(out, vec![0.5, 0.5]);
        assert_eq!(to_i16(2.0), 32767);
        assert_eq!(to_i16(-2.0), -32767);
    }
}
