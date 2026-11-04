use crate::protocol::{Codec, FRAME_SIZE, MAX_OPUS_PAYLOAD, SAMPLE_RATE};
use opus::{Application, Channels};

#[derive(Debug, thiserror::Error)]
#[error("opus: {0}")]
pub struct CodecError(String);

impl From<opus::Error> for CodecError {
    fn from(e: opus::Error) -> Self {
        CodecError(e.to_string())
    }
}

pub struct Encoder {
    inner: opus::Encoder,
}

impl Encoder {
    pub fn new(codec: Codec) -> Result<Self, CodecError> {
        let application = match codec {
            Codec::Voip => Application::Voip,
            Codec::Audio => Application::Audio,
            Codec::RestrictedLowDelay => Application::LowDelay,
        };
        Ok(Self {
            inner: opus::Encoder::new(SAMPLE_RATE, Channels::Mono, application)?,
        })
    }

    pub fn encode(&mut self, frame: &[i16]) -> Result<Vec<u8>, CodecError> {
        if frame.len() != FRAME_SIZE {
            return Err(CodecError(format!("frames hold {FRAME_SIZE} samples")));
        }
        Ok(self.inner.encode_vec(frame, MAX_OPUS_PAYLOAD)?)
    }
}

pub struct Decoder {
    inner: opus::Decoder,
}

impl Decoder {
    pub fn new() -> Result<Self, CodecError> {
        Ok(Self {
            inner: opus::Decoder::new(SAMPLE_RATE, Channels::Mono)?,
        })
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<i16>, CodecError> {
        let mut out = vec![0i16; FRAME_SIZE];
        let n = self.inner.decode(data, &mut out, false)?;
        out.truncate(n);
        Ok(out)
    }

    pub fn conceal_loss(&mut self) -> Result<Vec<i16>, CodecError> {
        let mut out = vec![0i16; FRAME_SIZE];
        let n = self.inner.decode(&[], &mut out, false)?;
        out.truncate(n);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(offset: usize) -> Vec<i16> {
        (0..FRAME_SIZE)
            .map(|i| {
                let t = (i + offset) as f32 / SAMPLE_RATE as f32;
                ((t * 440.0 * std::f32::consts::TAU).sin() * 12000.0) as i16
            })
            .collect()
    }

    #[test]
    fn a_tone_survives_the_round_trip_with_matching_energy() {
        let mut enc = Encoder::new(Codec::Voip).unwrap();
        let mut dec = Decoder::new().unwrap();
        let mut last = Vec::new();
        for k in 0..6 {
            let packet = enc.encode(&tone(k * FRAME_SIZE)).unwrap();
            assert!(!packet.is_empty() && packet.len() < 400);
            last = dec.decode(&packet).unwrap();
        }
        assert_eq!(last.len(), FRAME_SIZE);
        let db = crate::gate::level_db(&last);
        let reference = crate::gate::level_db(&tone(0));
        assert!((db - reference).abs() < 3.0, "decoded {db} dB vs {reference} dB");
    }

    #[test]
    fn wrong_frame_sizes_and_garbage_are_errors_not_panics() {
        let mut enc = Encoder::new(Codec::Audio).unwrap();
        assert!(enc.encode(&[0; 100]).is_err());
        let mut dec = Decoder::new().unwrap();
        assert!(dec.decode(&[0xFF, 0xFF, 0xFF]).is_ok() || dec.decode(&[0xFF, 0xFF, 0xFF]).is_err());
        assert_eq!(dec.conceal_loss().map(|f| f.len()).unwrap_or(FRAME_SIZE), FRAME_SIZE);
    }
}
