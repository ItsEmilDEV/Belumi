use crate::{
    mixer::Mixer,
    protocol::{FRAME_SIZE, SAMPLE_RATE},
    resample::{Resampler, downmix, to_i16},
};
use cpal::{
    SampleFormat,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Sender, SyncSender, channel},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("no {0} device found")]
    Missing(&'static str),
    #[error("{0}")]
    Backend(String),
}

pub struct Running {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn spawn_stream(
    build: impl FnOnce() -> Result<cpal::Stream, DeviceError> + Send + 'static,
) -> Result<Running, DeviceError> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let (ready, started) = channel::<Result<(), DeviceError>>();
    let thread = thread::spawn(move || {
        let stream = match build() {
            Ok(s) => s,
            Err(e) => {
                let _ = ready.send(Err(e));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = ready.send(Err(DeviceError::Backend(e.to_string())));
            return;
        }
        let _ = ready.send(Ok(()));
        while !flag.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(40));
        }
    });
    match started.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(Running {
            stop,
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(DeviceError::Backend("the audio device did not start".into())),
    }
}

pub fn start_capture(frames: SyncSender<Vec<i16>>) -> Result<Running, DeviceError> {
    spawn_stream(move || {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(DeviceError::Missing("microphone"))?;
        let config = device
            .default_input_config()
            .map_err(|e| DeviceError::Backend(e.to_string()))?;
        let channels = usize::from(config.channels());
        let rate = config.sample_rate().0;
        let format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        let mut state = CaptureState::new(rate, channels, frames);
        let err = |e| log::warn!(target: "belumi::voice", "microphone error: {e}");
        let stream = match format {
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |d: &[f32], _| state.feed(d),
                err,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |d: &[i16], _| {
                    let v: Vec<f32> = d.iter().map(|s| f32::from(*s) / 32768.0).collect();
                    state.feed(&v);
                },
                err,
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                &stream_config,
                move |d: &[u16], _| {
                    let v: Vec<f32> = d.iter().map(|s| (f32::from(*s) - 32768.0) / 32768.0).collect();
                    state.feed(&v);
                },
                err,
                None,
            ),
            other => {
                return Err(DeviceError::Backend(format!(
                    "unsupported microphone format {other:?}"
                )));
            }
        };
        stream.map_err(|e| DeviceError::Backend(e.to_string()))
    })
}

struct CaptureState {
    channels: usize,
    resampler: Resampler,
    pending: Vec<f32>,
    frames: SyncSender<Vec<i16>>,
}

impl CaptureState {
    fn new(rate: u32, channels: usize, frames: SyncSender<Vec<i16>>) -> Self {
        Self {
            channels,
            resampler: Resampler::new(rate, SAMPLE_RATE, 1),
            pending: Vec::new(),
            frames,
        }
    }

    fn feed(&mut self, data: &[f32]) {
        let mut mono = Vec::with_capacity(data.len() / self.channels.max(1));
        downmix(data, self.channels, &mut mono);
        self.resampler.process(&mono, &mut self.pending);
        while self.pending.len() >= FRAME_SIZE {
            let frame: Vec<i16> = self.pending.drain(..FRAME_SIZE).map(to_i16).collect();
            let _ = self.frames.try_send(frame);
        }
    }
}

pub fn start_playback(
    mixer: Arc<Mutex<Mixer>>,
    done: Sender<()>,
) -> Result<Running, DeviceError> {
    let running = spawn_stream(move || {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(DeviceError::Missing("speaker"))?;
        let config = device
            .default_output_config()
            .map_err(|e| DeviceError::Backend(e.to_string()))?;
        let channels = usize::from(config.channels());
        let rate = config.sample_rate().0;
        let format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        let mut player = Player::new(mixer, rate, channels);
        let err = |e| log::warn!(target: "belumi::voice", "speaker error: {e}");
        let stream = match format {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |d: &mut [f32], _| player.fill(d),
                err,
                None,
            ),
            SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |d: &mut [i16], _| {
                    let mut tmp = vec![0.0f32; d.len()];
                    player.fill(&mut tmp);
                    for (o, s) in d.iter_mut().zip(tmp) {
                        *o = to_i16(s);
                    }
                },
                err,
                None,
            ),
            other => {
                return Err(DeviceError::Backend(format!(
                    "unsupported speaker format {other:?}"
                )));
            }
        };
        stream.map_err(|e| DeviceError::Backend(e.to_string()))
    });
    let _ = done.send(());
    running
}

struct Player {
    mixer: Arc<Mutex<Mixer>>,
    channels: usize,
    resampler: Resampler,
    ready: Vec<f32>,
}

impl Player {
    fn new(mixer: Arc<Mutex<Mixer>>, rate: u32, channels: usize) -> Self {
        Self {
            mixer,
            channels: channels.max(1),
            resampler: Resampler::new(SAMPLE_RATE, rate, 2),
            ready: Vec::new(),
        }
    }

    fn fill(&mut self, out: &mut [f32]) {
        let frames_needed = out.len() / self.channels;
        while self.ready.len() / 2 < frames_needed {
            let mut chunk = vec![0.0f32; 480 * 2];
            if let Ok(mut m) = self.mixer.lock() {
                m.pull(&mut chunk);
            }
            self.resampler.process(&chunk, &mut self.ready);
        }
        for (i, frame) in out.chunks_exact_mut(self.channels).enumerate() {
            let (l, r) = (self.ready[i * 2], self.ready[i * 2 + 1]);
            for (c, o) in frame.iter_mut().enumerate() {
                *o = match c {
                    0 => l,
                    1 => r,
                    _ => 0.0,
                };
            }
        }
        self.ready.drain(..frames_needed * 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;

    #[test]
    fn captured_audio_becomes_whole_48k_mono_frames() {
        let (tx, rx) = sync_channel(16);
        let mut c = CaptureState::new(44_100, 2, tx);
        let stereo: Vec<f32> = (0..4410 * 2).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
        for _ in 0..4 {
            c.feed(&stereo);
        }
        let mut got = 0;
        while let Ok(frame) = rx.try_recv() {
            assert_eq!(frame.len(), FRAME_SIZE);
            assert!(frame.iter().all(|s| s.abs() < 40));
            got += 1;
        }
        assert!(got >= 8, "got {got} frames");
    }

    #[test]
    fn the_player_fills_the_device_buffer_from_the_mixer_and_pads_silence() {
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        mixer.lock().unwrap().push(1, &[16384; 480], (1.0, 1.0), 1.0);
        let mut p = Player::new(mixer, 48_000, 2);
        let mut out = vec![9.0f32; 960 * 2];
        p.fill(&mut out);
        assert!(out[..8].iter().all(|v| (*v - 0.5).abs() < 1e-3));
        assert!(out[out.len() - 8..].iter().all(|v| *v == 0.0));
    }
}
