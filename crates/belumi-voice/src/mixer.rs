use crate::protocol::{SAMPLE_RATE, Uuid};
use std::collections::HashMap;

const MAX_LATENCY_SAMPLES: usize = (SAMPLE_RATE as usize / 5) * 2;

#[derive(Default)]
pub struct Mixer {
    voices: HashMap<Uuid, Vec<f32>>,
}

impl Mixer {
    pub fn push(&mut self, voice: Uuid, mono: &[i16], gains: (f32, f32), volume: f32) {
        let queue = self.voices.entry(voice).or_default();
        for s in mono {
            let v = f32::from(*s) / 32768.0 * volume;
            queue.push(v * gains.0);
            queue.push(v * gains.1);
        }
        if queue.len() > MAX_LATENCY_SAMPLES {
            let drop = queue.len() - MAX_LATENCY_SAMPLES;
            queue.drain(..drop);
        }
    }

    pub fn pull(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        for queue in self.voices.values_mut() {
            let n = queue.len().min(out.len());
            for (o, s) in out.iter_mut().zip(queue.drain(..n)) {
                *o += s;
            }
        }
        for o in out.iter_mut() {
            *o = o.clamp(-1.0, 1.0);
        }
        self.voices.retain(|_, q| !q.is_empty());
    }

    pub fn speaking(&self) -> Vec<Uuid> {
        self.voices.keys().copied().collect()
    }

    pub fn clear(&mut self) {
        self.voices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voices_sum_and_drain_in_order() {
        let mut m = Mixer::default();
        m.push(1, &[16384, 16384], (1.0, 0.0), 1.0);
        m.push(2, &[16384, 16384], (0.0, 1.0), 1.0);
        let mut out = [0.0f32; 4];
        m.pull(&mut out);
        assert!((out[0] - 0.5).abs() < 1e-4 && (out[1] - 0.5).abs() < 1e-4);
        assert!((out[2] - 0.5).abs() < 1e-4 && (out[3] - 0.5).abs() < 1e-4);
        assert!(m.speaking().is_empty());
    }

    #[test]
    fn short_queues_pad_with_silence_and_output_is_clamped() {
        let mut m = Mixer::default();
        m.push(1, &[32767], (1.0, 1.0), 4.0);
        let mut out = [0.0f32; 6];
        m.pull(&mut out);
        assert_eq!(out[0], 1.0);
        assert_eq!(&out[2..], &[0.0; 4]);
    }

    #[test]
    fn a_stalled_listener_never_builds_more_than_a_fifth_of_a_second() {
        let mut m = Mixer::default();
        for _ in 0..50 {
            m.push(1, &[1000; 960], (1.0, 1.0), 1.0);
        }
        let queued = m.voices[&1].len();
        assert_eq!(queued, MAX_LATENCY_SAMPLES);
    }
}
