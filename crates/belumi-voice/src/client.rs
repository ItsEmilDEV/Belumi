use crate::{
    codec::{Decoder, Encoder},
    device::{self, DeviceError, Running},
    gate::{Gate, Mode, apply_gain, level_db},
    mixer::Mixer,
    protocol::{
        Init, Packet, Sound, Uuid, client_datagram, read_server_datagram,
    },
    spatial::{Listener, distance_gain, stereo_gains},
};
use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{Receiver, SyncSender, channel, sync_channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Connecting,
    Authenticated,
    Connected,
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceSettings {
    pub mode: Mode,
    pub threshold_db: f32,
    pub mic_gain: f32,
    pub volume: f32,
    pub muted: bool,
    pub deafened: bool,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            mode: Mode::PushToTalk,
            threshold_db: -50.0,
            mic_gain: 1.0,
            volume: 1.0,
            muted: false,
            deafened: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMode {
    Devices,
    None,
}

#[derive(Default)]
struct World {
    listener: Option<Listener>,
    players: HashMap<Uuid, [f64; 3]>,
}

struct Shared {
    init: Init,
    state: Mutex<State>,
    settings: Mutex<VoiceSettings>,
    talking: AtomicBool,
    stop: AtomicBool,
    world: Mutex<World>,
    mixer: Arc<Mutex<Mixer>>,
    mic_level_bits: AtomicU32,
    sent: AtomicU32,
    received: AtomicU32,
    notes: Mutex<Vec<String>>,
}

impl Shared {
    fn set_state(&self, s: State) {
        if let Ok(mut st) = self.state.lock() {
            if !matches!(*st, State::Failed(_)) || matches!(s, State::Failed(_)) {
                *st = s;
            }
        }
    }
    fn note(&self, text: String) {
        if let Ok(mut n) = self.notes.lock() {
            if n.len() < 16 {
                n.push(text);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("could not resolve the voice server {0:?}")]
    Resolve(String),
    #[error("socket: {0}")]
    Io(#[from] std::io::Error),
    #[error("opus: {0}")]
    Codec(#[from] crate::codec::CodecError),
}

pub fn voice_address(init: &Init, server_host: &str) -> Result<SocketAddr, VoiceError> {
    let host = init.voice_host.trim();
    let target = if host.is_empty() {
        format!("{}:{}", bare_host(server_host), init.server_port)
    } else if has_port(host) {
        host.to_owned()
    } else {
        format!("{}:{}", bracket(host), init.server_port)
    };
    target
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .ok_or(VoiceError::Resolve(target))
}

fn has_port(host: &str) -> bool {
    let Some((head, tail)) = host.rsplit_once(':') else {
        return false;
    };
    let single_colon = !head.contains(':') || head.ends_with(']');
    single_colon && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
}

fn bracket(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_owned()
    }
}

fn bare_host(address: &str) -> String {
    let a = address.trim();
    if let Some(rest) = a.strip_prefix('[')
        && let Some((h, _)) = rest.split_once(']')
    {
        return bracket(h);
    }
    match a.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') && p.chars().all(|c| c.is_ascii_digit()) => h.to_owned(),
        _ => bracket(a),
    }
}

pub struct VoiceClient {
    shared: Arc<Shared>,
    mic_in: SyncSender<Vec<i16>>,
    threads: Vec<JoinHandle<()>>,
    _devices: Vec<Running>,
}

impl VoiceClient {
    pub fn start(
        init: Init,
        server_host: &str,
        audio: AudioMode,
        settings: VoiceSettings,
    ) -> Result<Self, VoiceError> {
        let addr = voice_address(&init, server_host)?;
        let socket = UdpSocket::bind(if addr.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" })?;
        socket.set_read_timeout(Some(Duration::from_millis(40)))?;
        let shared = Arc::new(Shared {
            init,
            state: Mutex::new(State::Connecting),
            settings: Mutex::new(settings),
            talking: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            world: Mutex::new(World::default()),
            mixer: Arc::new(Mutex::new(Mixer::default())),
            mic_level_bits: AtomicU32::new((-127.0f32).to_bits()),
            sent: AtomicU32::new(0),
            received: AtomicU32::new(0),
            notes: Mutex::new(Vec::new()),
        });
        let socket = Arc::new(socket);
        let (mic_tx, mic_rx) = sync_channel::<Vec<i16>>(32);
        let mut devices = Vec::new();
        if audio == AudioMode::Devices {
            match device::start_capture(mic_tx.clone()) {
                Ok(r) => devices.push(r),
                Err(e) => shared.note(format!("Microphone unavailable: {e}")),
            }
            let (done, _) = channel();
            match device::start_playback(shared.mixer.clone(), done) {
                Ok(r) => devices.push(r),
                Err(e) => shared.note(format!("Speaker unavailable: {e}")),
            }
        }
        let net = {
            let (s, sock) = (shared.clone(), socket.clone());
            thread::Builder::new()
                .name("belumi-voice-net".into())
                .spawn(move || net_loop(&s, &sock, addr))?
        };
        let mic = {
            let (s, sock) = (shared.clone(), socket.clone());
            thread::Builder::new()
                .name("belumi-voice-mic".into())
                .spawn(move || mic_loop(&s, &sock, addr, mic_rx))?
        };
        Ok(Self {
            shared,
            mic_in: mic_tx,
            threads: vec![net, mic],
            _devices: devices,
        })
    }

    pub fn state(&self) -> State {
        self.shared.state.lock().map(|s| s.clone()).unwrap_or(State::Connecting)
    }

    pub fn set_talking(&self, on: bool) {
        self.shared.talking.store(on, Ordering::Relaxed);
    }

    pub fn set_settings(&self, s: VoiceSettings) {
        if let Ok(mut cur) = self.shared.settings.lock() {
            *cur = s;
        }
    }

    pub fn update_world(&self, listener: Listener, players: Vec<(Uuid, [f64; 3])>) {
        if let Ok(mut w) = self.shared.world.lock() {
            w.listener = Some(listener);
            w.players = players.into_iter().collect();
        }
    }

    pub fn speaking(&self) -> Vec<Uuid> {
        self.shared.mixer.lock().map(|m| m.speaking()).unwrap_or_default()
    }

    pub fn mic_level_db(&self) -> f32 {
        f32::from_bits(self.shared.mic_level_bits.load(Ordering::Relaxed))
    }

    pub fn take_notes(&self) -> Vec<String> {
        self.shared
            .notes
            .lock()
            .map(|mut n| std::mem::take(&mut *n))
            .unwrap_or_default()
    }

    pub fn packets(&self) -> (u32, u32) {
        (
            self.shared.sent.load(Ordering::Relaxed),
            self.shared.received.load(Ordering::Relaxed),
        )
    }

    pub fn feed_mic(&self, frame: Vec<i16>) {
        let _ = self.mic_in.try_send(frame);
    }

    pub fn pull_playback(&self, out: &mut [f32]) {
        if let Ok(mut m) = self.shared.mixer.lock() {
            m.pull(out);
        }
    }

    pub fn player(&self) -> Uuid {
        self.shared.init.player
    }

    pub fn stop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        self._devices.clear();
        if let Ok(mut m) = self.shared.mixer.lock() {
            m.clear();
        }
    }
}

impl Drop for VoiceClient {
    fn drop(&mut self) {
        self.stop();
    }
}

fn send(shared: &Shared, socket: &UdpSocket, addr: SocketAddr, packet: &Packet) {
    if socket.send_to(&client_datagram(&shared.init, packet), addr).is_ok() {
        shared.sent.fetch_add(1, Ordering::Relaxed);
    }
}

fn net_loop(shared: &Shared, socket: &UdpSocket, addr: SocketAddr) {
    let mut buf = vec![0u8; crate::protocol::MAX_PACKET + 64];
    let mut nonce: Option<[u8; crate::protocol::NONCE_SIZE]> = None;
    let mut last_hello: Option<Instant> = None;
    let mut last_keepalive = Instant::now();
    let mut connected = false;
    let mut channels: HashMap<Uuid, Channel> = HashMap::new();
    let timeout = Duration::from_millis(u64::try_from(shared.init.keep_alive_ms.max(500)).unwrap_or(1000) * 10);
    while !shared.stop.load(Ordering::Relaxed) {
        if !connected && last_hello.is_none_or(|t| t.elapsed() >= Duration::from_millis(1000)) {
            last_hello = Some(Instant::now());
            match nonce {
                None => send(shared, socket, addr, &Packet::Authenticate),
                Some(n) => send(shared, socket, addr, &Packet::ConnectionCheck(n)),
            }
        }
        if connected && last_keepalive.elapsed() > timeout {
            shared.set_state(State::Failed("The voice server stopped answering.".into()));
            return;
        }
        let Ok((n, from)) = socket.recv_from(&mut buf).map_err(|_| ()) else {
            continue;
        };
        if from.ip() != addr.ip() {
            continue;
        }
        let Ok(packet) = read_server_datagram(&shared.init, &buf[..n]) else {
            continue;
        };
        shared.received.fetch_add(1, Ordering::Relaxed);
        match packet {
            Packet::AuthenticateAck(n) => {
                if nonce.is_none() {
                    nonce = Some(n);
                    last_hello = None;
                    shared.set_state(State::Authenticated);
                }
            }
            Packet::ConnectionCheckAck => {
                if !connected {
                    connected = true;
                    last_keepalive = Instant::now();
                    shared.set_state(State::Connected);
                }
            }
            Packet::Ping { id, timestamp } => {
                send(shared, socket, addr, &Packet::Ping { id, timestamp });
            }
            Packet::KeepAlive => {
                last_keepalive = Instant::now();
                send(shared, socket, addr, &Packet::KeepAlive);
            }
            Packet::Sound(sound) => {
                if connected {
                    play(shared, &mut channels, &sound);
                }
            }
            _ => {}
        }
    }
}

struct Channel {
    decoder: Decoder,
    last_sequence: i64,
    last_heard: Instant,
}

fn play(shared: &Shared, channels: &mut HashMap<Uuid, Channel>, sound: &Sound) {
    let settings = shared.settings.lock().map(|s| *s).unwrap_or_default();
    if settings.deafened {
        return;
    }
    channels.retain(|_, c| c.last_heard.elapsed() < Duration::from_secs(30));
    let id = sound.channel();
    if sound.data().is_empty() {
        channels.remove(&id);
        return;
    }
    let ch = match channels.get_mut(&id) {
        Some(c) => c,
        None => {
            let Ok(decoder) = Decoder::new() else {
                return;
            };
            channels.entry(id).or_insert(Channel {
                decoder,
                last_sequence: sound.sequence() - 1,
                last_heard: Instant::now(),
            })
        }
    };
    if sound.sequence() <= ch.last_sequence {
        return;
    }
    let gap = sound.sequence() - ch.last_sequence - 1;
    if (1..=3).contains(&gap) {
        for _ in 0..gap {
            if let Ok(f) = ch.decoder.conceal_loss() {
                enqueue(shared, sound, &f, settings.volume);
            }
        }
    }
    ch.last_sequence = sound.sequence();
    ch.last_heard = Instant::now();
    match ch.decoder.decode(sound.data()) {
        Ok(frame) => enqueue(shared, sound, &frame, settings.volume),
        Err(e) => log::debug!(target: "belumi::voice", "bad audio frame: {e}"),
    }
}

fn enqueue(shared: &Shared, sound: &Sound, frame: &[i16], volume: f32) {
    let world = shared.world.lock();
    let (listener, known) = match &world {
        Ok(w) => (w.listener, w.players.get(&sound.sender()).copied()),
        Err(_) => (None, None),
    };
    let (gains, atten) = match (sound, listener) {
        (Sound::Group { .. }, _) | (_, None) => ((1.0, 1.0), 1.0),
        (Sound::Player { distance, .. }, Some(l)) => match known {
            Some(p) => (stereo_gains(&l, p), distance_gain(max_distance(*distance, &shared.init), &l, p)),
            None => ((1.0, 1.0), 1.0),
        },
        (Sound::Location { at, distance, .. }, Some(l)) => (
            stereo_gains(&l, *at),
            distance_gain(max_distance(*distance, &shared.init), &l, *at),
        ),
    };
    drop(world);
    if let Ok(mut m) = shared.mixer.lock() {
        m.push(sound.channel(), frame, gains, volume * atten);
    }
}

fn max_distance(packet_distance: f32, init: &Init) -> f32 {
    if packet_distance > 0.0 {
        packet_distance
    } else {
        init.distance as f32
    }
}

fn mic_loop(shared: &Shared, socket: &UdpSocket, addr: SocketAddr, frames: Receiver<Vec<i16>>) {
    let Ok(mut encoder) = Encoder::new(shared.init.codec) else {
        shared.note("Could not start the Opus encoder.".into());
        return;
    };
    let mut gate = Gate::default();
    let mut sequence: i64 = 0;
    let mut open = false;
    while !shared.stop.load(Ordering::Relaxed) {
        let Ok(mut frame) = frames.recv_timeout(Duration::from_millis(50)) else {
            continue;
        };
        let settings = shared.settings.lock().map(|s| *s).unwrap_or_default();
        apply_gain(&mut frame, settings.mic_gain);
        shared
            .mic_level_bits
            .store(level_db(&frame).to_bits(), Ordering::Relaxed);
        let connected = matches!(shared.state.lock().map(|s| s.clone()), Ok(State::Connected));
        let talk = shared.talking.load(Ordering::Relaxed);
        let want = connected && !settings.muted && gate.open(&frame, settings.mode, talk, settings.threshold_db);
        if want {
            if let Ok(data) = encoder.encode(&frame) {
                send(
                    shared,
                    socket,
                    addr,
                    &Packet::Mic {
                        data,
                        whispering: false,
                        sequence,
                    },
                );
                sequence += 1;
                open = true;
            }
        } else if open {
            send(
                shared,
                socket,
                addr,
                &Packet::Mic {
                    data: Vec::new(),
                    whispering: false,
                    sequence,
                },
            );
            sequence += 1;
            open = false;
        }
    }
}

#[allow(dead_code)]
fn _assert_device_error_is_send(e: DeviceError) -> impl Send {
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        FRAME_SIZE, read_client_datagram, sample_init, server_datagram,
    };

    fn tone(offset: usize) -> Vec<i16> {
        (0..FRAME_SIZE)
            .map(|i| (((i + offset) as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 12000.0) as i16)
            .collect()
    }

    struct Fake {
        socket: UdpSocket,
        init: Init,
    }

    impl Fake {
        fn new() -> Self {
            let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
            socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
            let mut init = sample_init();
            init.server_port = socket.local_addr().unwrap().port();
            init.voice_host = String::new();
            init.keep_alive_ms = 500;
            Self { socket, init }
        }
        fn read(&self) -> Option<(SocketAddr, Packet)> {
            let mut buf = [0u8; 4096];
            let (n, from) = self.socket.recv_from(&mut buf).ok()?;
            let id = self.init.connection_id;
            let secret = self.init.secret;
            let (_, p) = read_client_datagram(|i| (i == id).then_some(secret), &buf[..n]).ok()?;
            Some((from, p))
        }
        fn send(&self, to: SocketAddr, p: &Packet) {
            self.socket.send_to(&server_datagram(&self.init, p), to).unwrap();
        }
    }

    fn wait_for(client: &VoiceClient, want: State) {
        for _ in 0..100 {
            if client.state() == want {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("state is {:?}, wanted {want:?}", client.state());
    }

    #[test]
    fn the_client_authenticates_connects_plays_sound_and_sends_the_microphone() {
        let fake = Fake::new();
        let client = VoiceClient::start(fake.init.clone(), "127.0.0.1:25565", AudioMode::None, VoiceSettings::default()).unwrap();
        let mut peer = None;
        for _ in 0..40 {
            if let Some((from, p)) = fake.read() {
                match p {
                    Packet::Authenticate => {
                        peer = Some(from);
                        fake.send(from, &Packet::AuthenticateAck([7; 16]));
                    }
                    Packet::ConnectionCheck(n) => {
                        assert_eq!(n, [7; 16]);
                        fake.send(from, &Packet::ConnectionCheckAck);
                        break;
                    }
                    _ => {}
                }
            }
        }
        let peer = peer.expect("the client said hello");
        wait_for(&client, State::Connected);

        let mut enc = Encoder::new(crate::protocol::Codec::Voip).unwrap();
        client.update_world(
            Listener { pos: [0.0, 64.0, 0.0], yaw: 0.0 },
            vec![(0xABCD, [-5.0, 64.0, 0.0])],
        );
        for k in 0..6 {
            let data = enc.encode(&tone(k * FRAME_SIZE)).unwrap();
            fake.send(peer, &Packet::Sound(Sound::Player {
                channel: 0xABCD,
                sender: 0xABCD,
                data,
                sequence: k as i64,
                distance: 48.0,
                whispering: false,
                category: None,
            }));
        }
        thread::sleep(Duration::from_millis(200));
        assert!(client.speaking().contains(&0xABCD));
        let mut out = vec![0.0f32; 960 * 2];
        client.pull_playback(&mut out);
        let (l, r): (f32, f32) = out.chunks(2).fold((0.0, 0.0), |a, f| (a.0 + f[0].abs(), a.1 + f[1].abs()));
        assert!(l + r > 10.0, "audio arrived");
        assert!(r > l, "a speaker to the west is on the right when facing south");

        client.set_talking(true);
        for k in 0..3 {
            client.feed_mic(tone(k * FRAME_SIZE));
        }
        let mut mic = 0;
        for _ in 0..30 {
            match fake.read() {
                Some((_, Packet::Mic { data, sequence, .. })) if !data.is_empty() => {
                    mic += 1;
                    assert!(sequence >= 0);
                    if mic == 3 {
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(mic, 3, "microphone frames reached the server");
    }

    #[test]
    fn a_silent_server_after_connecting_fails_the_connection() {
        let fake = Fake::new();
        let client = VoiceClient::start(fake.init.clone(), "127.0.0.1", AudioMode::None, VoiceSettings::default()).unwrap();
        for _ in 0..40 {
            if let Some((from, p)) = fake.read() {
                match p {
                    Packet::Authenticate => fake.send(from, &Packet::AuthenticateAck([1; 16])),
                    Packet::ConnectionCheck(_) => {
                        fake.send(from, &Packet::ConnectionCheckAck);
                        break;
                    }
                    _ => {}
                }
            }
        }
        wait_for(&client, State::Connected);
        for _ in 0..200 {
            if matches!(client.state(), State::Failed(_)) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("the connection never timed out: {:?}", client.state());
    }

    #[test]
    fn muted_clients_and_closed_gates_send_nothing() {
        let fake = Fake::new();
        let client = VoiceClient::start(
            fake.init.clone(),
            "127.0.0.1",
            AudioMode::None,
            VoiceSettings { muted: true, ..VoiceSettings::default() },
        )
        .unwrap();
        for _ in 0..40 {
            if let Some((from, p)) = fake.read() {
                match p {
                    Packet::Authenticate => fake.send(from, &Packet::AuthenticateAck([1; 16])),
                    Packet::ConnectionCheck(_) => {
                        fake.send(from, &Packet::ConnectionCheckAck);
                        break;
                    }
                    _ => {}
                }
            }
        }
        wait_for(&client, State::Connected);
        client.set_talking(true);
        client.feed_mic(tone(0));
        for _ in 0..5 {
            if let Some((_, Packet::Mic { .. })) = fake.read() {
                panic!("a muted client transmitted");
            }
        }
    }

    #[test]
    fn voice_hosts_resolve_with_and_without_ports() {
        let mut init = sample_init();
        init.server_port = 24454;
        init.voice_host = String::new();
        assert_eq!(voice_address(&init, "127.0.0.1:25565").unwrap().port(), 24454);
        init.voice_host = "127.0.0.1".into();
        assert_eq!(voice_address(&init, "x").unwrap().port(), 24454);
        init.voice_host = "127.0.0.1:30000".into();
        assert_eq!(voice_address(&init, "x").unwrap().port(), 30000);
        init.voice_host = "[::1]".into();
        assert_eq!(voice_address(&init, "x").unwrap().port(), 24454);
        init.voice_host = "[::1]:777".into();
        assert_eq!(voice_address(&init, "x").unwrap().port(), 777);
        init.voice_host = "no such host.invalid".into();
        assert!(voice_address(&init, "x").is_err());
    }
}
