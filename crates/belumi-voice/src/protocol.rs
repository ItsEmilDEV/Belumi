use aes_gcm::{
    Aes128Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};

pub const COMPATIBILITY_VERSION: i32 = 21;
pub const SECRET_CHANNEL: &str = "voicechat:secret";
pub const REQUEST_SECRET_CHANNEL: &str = "voicechat:request_secret";
pub const MAGIC: u8 = 0xFF;
pub const NONCE_SIZE: usize = 16;
pub const MAX_PACKET: usize = 2048;
pub const MAX_OPUS_PAYLOAD: usize = 1275;
pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_SIZE: usize = 960;

const IV_SIZE: usize = 12;
const WHISPER_MASK: u8 = 0b01;
const CATEGORY_MASK: u8 = 0b10;

pub type Uuid = u128;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("packet ends early")]
    Short,
    #[error("bad magic byte")]
    Magic,
    #[error("could not decrypt")]
    Crypto,
    #[error("unknown packet type {0}")]
    Kind(u8),
    #[error("field out of range")]
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    Voip,
    Audio,
    RestrictedLowDelay,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Init {
    pub secret: [u8; 16],
    pub connection_id: Uuid,
    pub server_port: u16,
    pub player: Uuid,
    pub codec: Codec,
    pub mtu: i32,
    pub distance: f64,
    pub keep_alive_ms: i32,
    pub groups: bool,
    pub voice_host: String,
    pub allow_recording: bool,
    pub legacy: bool,
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
        if self.0.len() < n {
            return Err(ProtocolError::Short);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, ProtocolError> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, ProtocolError> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, ProtocolError> {
        Ok(f32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, ProtocolError> {
        Ok(f64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn uuid(&mut self) -> Result<Uuid, ProtocolError> {
        Ok(u128::from_be_bytes(self.take(16)?.try_into().unwrap()))
    }
    fn varint(&mut self) -> Result<usize, ProtocolError> {
        let mut value = 0usize;
        for i in 0..5 {
            let b = self.u8()?;
            value |= usize::from(b & 0x7F) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ProtocolError::Range)
    }
    fn bytes(&mut self, max: usize) -> Result<Vec<u8>, ProtocolError> {
        let n = self.varint()?;
        if n > max {
            return Err(ProtocolError::Range);
        }
        Ok(self.take(n)?.to_vec())
    }
    fn utf(&mut self, max: usize) -> Result<String, ProtocolError> {
        let raw = self.bytes(max * 4)?;
        String::from_utf8(raw).map_err(|_| ProtocolError::Range)
    }
}

fn put_varint(out: &mut Vec<u8>, mut v: usize) {
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn put_bytes(out: &mut Vec<u8>, data: &[u8]) {
    put_varint(out, data.len());
    out.extend_from_slice(data);
}

fn put_utf(out: &mut Vec<u8>, s: &str) {
    put_bytes(out, s.as_bytes());
}

pub fn parse_secret(payload: &[u8]) -> Result<Init, ProtocolError> {
    parse_secret_with(payload, true).or_else(|first| parse_secret_with(payload, false).map_err(|_| first))
}

fn parse_secret_with(payload: &[u8], with_connection_id: bool) -> Result<Init, ProtocolError> {
    let mut r = Reader(payload);
    let secret: [u8; 16] = r.take(16)?.try_into().unwrap();
    let connection_id = if with_connection_id { Some(r.uuid()?) } else { None };
    let port = r.i32()?;
    let player = r.uuid()?;
    let connection_id = connection_id.unwrap_or(player);
    let codec = match r.u8()? {
        0 => Codec::Voip,
        1 => Codec::Audio,
        2 => Codec::RestrictedLowDelay,
        _ => return Err(ProtocolError::Range),
    };
    let mtu = r.i32()?.clamp(512, 2048);
    let distance = r.f64()?;
    let keep_alive_ms = r.i32()?;
    let groups = r.u8()? != 0;
    let voice_host = r.utf(256)?;
    let allow_recording = r.u8()? != 0;
    if !r.0.is_empty() {
        return Err(ProtocolError::Range);
    }
    Ok(Init {
        secret,
        connection_id,
        server_port: u16::try_from(port).map_err(|_| ProtocolError::Range)?,
        player,
        codec,
        mtu,
        distance,
        keep_alive_ms,
        groups,
        voice_host,
        allow_recording,
        legacy: !with_connection_id,
    })
}

pub fn encode_secret(init: &Init) -> Vec<u8> {
    let mut out = init.secret.to_vec();
    if !init.legacy {
        out.extend_from_slice(&init.connection_id.to_be_bytes());
    }
    out.extend_from_slice(&i32::from(init.server_port).to_be_bytes());
    out.extend_from_slice(&init.player.to_be_bytes());
    out.push(match init.codec {
        Codec::Voip => 0,
        Codec::Audio => 1,
        Codec::RestrictedLowDelay => 2,
    });
    out.extend_from_slice(&init.mtu.to_be_bytes());
    out.extend_from_slice(&init.distance.to_be_bytes());
    out.extend_from_slice(&init.keep_alive_ms.to_be_bytes());
    out.push(u8::from(init.groups));
    put_utf(&mut out, &init.voice_host);
    out.push(u8::from(init.allow_recording));
    out
}

pub fn request_secret_payload(version: i32) -> Vec<u8> {
    version.to_be_bytes().to_vec()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Sound {
    Player {
        channel: Uuid,
        sender: Uuid,
        data: Vec<u8>,
        sequence: i64,
        distance: f32,
        whispering: bool,
        category: Option<String>,
    },
    Group {
        channel: Uuid,
        sender: Uuid,
        data: Vec<u8>,
        sequence: i64,
        category: Option<String>,
    },
    Location {
        channel: Uuid,
        sender: Uuid,
        at: [f64; 3],
        data: Vec<u8>,
        sequence: i64,
        distance: f32,
        category: Option<String>,
    },
}

impl Sound {
    pub fn channel(&self) -> Uuid {
        match self {
            Sound::Player { channel, .. }
            | Sound::Group { channel, .. }
            | Sound::Location { channel, .. } => *channel,
        }
    }
    pub fn sender(&self) -> Uuid {
        match self {
            Sound::Player { sender, .. }
            | Sound::Group { sender, .. }
            | Sound::Location { sender, .. } => *sender,
        }
    }
    pub fn data(&self) -> &[u8] {
        match self {
            Sound::Player { data, .. } | Sound::Group { data, .. } | Sound::Location { data, .. } => {
                data
            }
        }
    }
    pub fn sequence(&self) -> i64 {
        match self {
            Sound::Player { sequence, .. }
            | Sound::Group { sequence, .. }
            | Sound::Location { sequence, .. } => *sequence,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Packet {
    Mic {
        data: Vec<u8>,
        whispering: bool,
        sequence: i64,
    },
    Sound(Sound),
    Authenticate,
    AuthenticateAck([u8; NONCE_SIZE]),
    Ping {
        id: Uuid,
        timestamp: i64,
    },
    KeepAlive,
    ConnectionCheck([u8; NONCE_SIZE]),
    ConnectionCheckAck,
}

fn category(r: &mut Reader<'_>, flags: u8) -> Result<Option<String>, ProtocolError> {
    if flags & CATEGORY_MASK != 0 {
        Ok(Some(r.utf(16)?))
    } else {
        Ok(None)
    }
}

fn put_category(flags: &mut u8, c: &Option<String>) {
    if c.is_some() {
        *flags |= CATEGORY_MASK;
    }
}

fn nonce(r: &mut Reader<'_>) -> Result<[u8; NONCE_SIZE], ProtocolError> {
    if r.0.is_empty() {
        return Ok([0; NONCE_SIZE]);
    }
    Ok(r.take(NONCE_SIZE)?.try_into().unwrap())
}

pub fn decode_packet(plain: &[u8]) -> Result<Packet, ProtocolError> {
    let mut r = Reader(plain);
    Ok(match r.u8()? {
        0x1 => {
            let data = r.bytes(MAX_OPUS_PAYLOAD)?;
            let sequence = r.i64()?;
            let whispering = r.u8()? != 0;
            Packet::Mic {
                data,
                whispering,
                sequence,
            }
        }
        0x2 => {
            let channel = r.uuid()?;
            let sender = r.uuid()?;
            let data = r.bytes(MAX_OPUS_PAYLOAD)?;
            let sequence = r.i64()?;
            let distance = r.f32()?;
            let flags = r.u8()?;
            Packet::Sound(Sound::Player {
                channel,
                sender,
                data,
                sequence,
                distance,
                whispering: flags & WHISPER_MASK != 0,
                category: category(&mut r, flags)?,
            })
        }
        0x3 => {
            let channel = r.uuid()?;
            let sender = r.uuid()?;
            let data = r.bytes(MAX_OPUS_PAYLOAD)?;
            let sequence = r.i64()?;
            let flags = r.u8()?;
            Packet::Sound(Sound::Group {
                channel,
                sender,
                data,
                sequence,
                category: category(&mut r, flags)?,
            })
        }
        0x4 => {
            let channel = r.uuid()?;
            let sender = r.uuid()?;
            let at = [r.f64()?, r.f64()?, r.f64()?];
            let data = r.bytes(MAX_OPUS_PAYLOAD)?;
            let sequence = r.i64()?;
            let distance = r.f32()?;
            let flags = r.u8()?;
            Packet::Sound(Sound::Location {
                channel,
                sender,
                at,
                data,
                sequence,
                distance,
                category: category(&mut r, flags)?,
            })
        }
        0x5 => Packet::Authenticate,
        0x6 => Packet::AuthenticateAck(nonce(&mut r)?),
        0x7 => Packet::Ping {
            id: r.uuid()?,
            timestamp: r.i64()?,
        },
        0x8 => Packet::KeepAlive,
        0x9 => Packet::ConnectionCheck(nonce(&mut r)?),
        0xA => Packet::ConnectionCheckAck,
        other => return Err(ProtocolError::Kind(other)),
    })
}

pub fn encode_packet(packet: &Packet) -> Vec<u8> {
    encode_packet_with(packet, true)
}

pub fn encode_packet_with(packet: &Packet, nonces: bool) -> Vec<u8> {
    let mut o = Vec::new();
    match packet {
        Packet::Mic {
            data,
            whispering,
            sequence,
        } => {
            o.push(0x1);
            put_bytes(&mut o, data);
            o.extend_from_slice(&sequence.to_be_bytes());
            o.push(u8::from(*whispering));
        }
        Packet::Sound(Sound::Player {
            channel,
            sender,
            data,
            sequence,
            distance,
            whispering,
            category,
        }) => {
            o.push(0x2);
            o.extend_from_slice(&channel.to_be_bytes());
            o.extend_from_slice(&sender.to_be_bytes());
            put_bytes(&mut o, data);
            o.extend_from_slice(&sequence.to_be_bytes());
            o.extend_from_slice(&distance.to_be_bytes());
            let mut flags = if *whispering { WHISPER_MASK } else { 0 };
            put_category(&mut flags, category);
            o.push(flags);
            if let Some(c) = category {
                put_utf(&mut o, c);
            }
        }
        Packet::Sound(Sound::Group {
            channel,
            sender,
            data,
            sequence,
            category,
        }) => {
            o.push(0x3);
            o.extend_from_slice(&channel.to_be_bytes());
            o.extend_from_slice(&sender.to_be_bytes());
            put_bytes(&mut o, data);
            o.extend_from_slice(&sequence.to_be_bytes());
            let mut flags = 0;
            put_category(&mut flags, category);
            o.push(flags);
            if let Some(c) = category {
                put_utf(&mut o, c);
            }
        }
        Packet::Sound(Sound::Location {
            channel,
            sender,
            at,
            data,
            sequence,
            distance,
            category,
        }) => {
            o.push(0x4);
            o.extend_from_slice(&channel.to_be_bytes());
            o.extend_from_slice(&sender.to_be_bytes());
            for c in at {
                o.extend_from_slice(&c.to_be_bytes());
            }
            put_bytes(&mut o, data);
            o.extend_from_slice(&sequence.to_be_bytes());
            o.extend_from_slice(&distance.to_be_bytes());
            let mut flags = 0;
            put_category(&mut flags, category);
            o.push(flags);
            if let Some(c) = category {
                put_utf(&mut o, c);
            }
        }
        Packet::Authenticate => o.push(0x5),
        Packet::AuthenticateAck(n) => {
            o.push(0x6);
            if nonces {
                o.extend_from_slice(n);
            }
        }
        Packet::Ping { id, timestamp } => {
            o.push(0x7);
            o.extend_from_slice(&id.to_be_bytes());
            o.extend_from_slice(&timestamp.to_be_bytes());
        }
        Packet::KeepAlive => o.push(0x8),
        Packet::ConnectionCheck(n) => {
            o.push(0x9);
            if nonces {
                o.extend_from_slice(n);
            }
        }
        Packet::ConnectionCheckAck => o.push(0xA),
    }
    o
}

pub fn encrypt(secret: &[u8; 16], plain: &[u8]) -> Vec<u8> {
    let cipher = Aes128Gcm::new(secret.into());
    let mut iv = [0u8; IV_SIZE];
    OsRng.fill_bytes(&mut iv);
    let body = cipher
        .encrypt(Nonce::from_slice(&iv), plain)
        .expect("AES-GCM encryption does not fail for in-memory buffers");
    let mut out = iv.to_vec();
    out.extend_from_slice(&body);
    out
}

pub fn decrypt(secret: &[u8; 16], payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.len() <= IV_SIZE {
        return Err(ProtocolError::Short);
    }
    let (iv, body) = payload.split_at(IV_SIZE);
    Aes128Gcm::new(secret.into())
        .decrypt(Nonce::from_slice(iv), body)
        .map_err(|_| ProtocolError::Crypto)
}

pub fn client_datagram(init: &Init, packet: &Packet) -> Vec<u8> {
    let plain = if init.legacy && matches!(packet, Packet::Authenticate) {
        let mut o = vec![0x5];
        o.extend_from_slice(&init.player.to_be_bytes());
        o.extend_from_slice(&init.secret);
        o
    } else {
        encode_packet_with(packet, !init.legacy)
    };
    let sealed = encrypt(&init.secret, &plain);
    let mut out = vec![MAGIC];
    out.extend_from_slice(&init.connection_id.to_be_bytes());
    put_bytes(&mut out, &sealed);
    out
}

pub fn read_server_datagram(init: &Init, datagram: &[u8]) -> Result<Packet, ProtocolError> {
    let mut r = Reader(datagram);
    if r.u8()? != MAGIC {
        return Err(ProtocolError::Magic);
    }
    let sealed = r.bytes(MAX_PACKET)?;
    decode_packet(&decrypt(&init.secret, &sealed)?)
}

pub fn server_datagram(init: &Init, packet: &Packet) -> Vec<u8> {
    let sealed = encrypt(&init.secret, &encode_packet_with(packet, !init.legacy));
    let mut out = vec![MAGIC];
    put_bytes(&mut out, &sealed);
    out
}

pub fn read_client_datagram(
    secret_of: impl Fn(Uuid) -> Option<[u8; 16]>,
    datagram: &[u8],
) -> Result<(Uuid, Packet), ProtocolError> {
    let mut r = Reader(datagram);
    if r.u8()? != MAGIC {
        return Err(ProtocolError::Magic);
    }
    let id = r.uuid()?;
    let secret = secret_of(id).ok_or(ProtocolError::Crypto)?;
    let sealed = r.bytes(MAX_PACKET)?;
    Ok((id, decode_packet(&decrypt(&secret, &sealed)?)?))
}

#[cfg(test)]
pub(crate) fn sample_init() -> Init {
    Init {
        secret: *b"0123456789abcdef",
        connection_id: 0x1111_2222_3333_4444_5555_6666_7777_8888,
        server_port: 24454,
        player: 0xAAAA_BBBB_CCCC_DDDD_EEEE_FFFF_0000_1111,
        codec: Codec::Voip,
        mtu: 1024,
        distance: 48.0,
        keep_alive_ms: 1000,
        groups: true,
        voice_host: "voice.example.org:24454".into(),
        allow_recording: true,
        legacy: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_packet_round_trips_and_clamps_the_mtu() {
        let init = sample_init();
        assert_eq!(parse_secret(&encode_secret(&init)).unwrap(), init);
        let mut small = init.clone();
        small.mtu = 5;
        assert_eq!(parse_secret(&encode_secret(&small)).unwrap().mtu, 512);
        assert_eq!(parse_secret(&[0; 5]), Err(ProtocolError::Short));
        let mut real = init.secret.to_vec();
        real.extend_from_slice(&i32::from(init.server_port).to_be_bytes());
        real.extend_from_slice(&init.player.to_be_bytes());
        real.push(0);
        real.extend_from_slice(&init.mtu.to_be_bytes());
        real.extend_from_slice(&init.distance.to_be_bytes());
        real.extend_from_slice(&init.keep_alive_ms.to_be_bytes());
        real.push(1);
        put_utf(&mut real, &init.voice_host);
        real.push(1);
        let parsed = parse_secret(&real).unwrap();
        assert_eq!(parsed.connection_id, init.player);
        assert_eq!(parsed.server_port, init.server_port);
        assert_eq!(parsed.voice_host, init.voice_host);
    }

    #[test]
    fn request_secret_is_the_version_as_a_big_endian_int() {
        assert_eq!(request_secret_payload(21), vec![0, 0, 0, 21]);
    }

    #[test]
    fn every_packet_kind_round_trips() {
        let packets = vec![
            Packet::Mic {
                data: vec![1, 2, 3],
                whispering: true,
                sequence: 77,
            },
            Packet::Sound(Sound::Player {
                channel: 5,
                sender: 6,
                data: vec![9; 40],
                sequence: -3,
                distance: 48.5,
                whispering: true,
                category: Some("radio".into()),
            }),
            Packet::Sound(Sound::Group {
                channel: 1,
                sender: 2,
                data: vec![],
                sequence: 4,
                category: None,
            }),
            Packet::Sound(Sound::Location {
                channel: 1,
                sender: 2,
                at: [1.5, -64.0, 300.25],
                data: vec![7],
                sequence: 4,
                distance: 12.0,
                category: None,
            }),
            Packet::Authenticate,
            Packet::AuthenticateAck([3; NONCE_SIZE]),
            Packet::Ping {
                id: 42,
                timestamp: 1_700_000_000_000,
            },
            Packet::KeepAlive,
            Packet::ConnectionCheck([9; NONCE_SIZE]),
            Packet::ConnectionCheckAck,
        ];
        for p in packets {
            assert_eq!(decode_packet(&encode_packet(&p)).unwrap(), p);
        }
        assert_eq!(decode_packet(&[0x55]), Err(ProtocolError::Kind(0x55)));
        assert_eq!(decode_packet(&[]), Err(ProtocolError::Short));
    }

    #[test]
    fn datagrams_are_sealed_and_bound_to_the_connection() {
        let init = sample_init();
        let wire = client_datagram(&init, &Packet::KeepAlive);
        assert_eq!(wire[0], MAGIC);
        let (id, p) = read_client_datagram(|i| (i == init.connection_id).then_some(init.secret), &wire).unwrap();
        assert_eq!((id, p), (init.connection_id, Packet::KeepAlive));
        assert!(read_client_datagram(|_| None, &wire).is_err());

        let back = server_datagram(&init, &Packet::ConnectionCheckAck);
        assert_eq!(read_server_datagram(&init, &back).unwrap(), Packet::ConnectionCheckAck);
        let mut other = init.clone();
        other.secret = [7; 16];
        assert_eq!(read_server_datagram(&other, &back), Err(ProtocolError::Crypto));
        let mut tampered = back.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        assert_eq!(read_server_datagram(&init, &tampered), Err(ProtocolError::Crypto));
        assert_eq!(read_server_datagram(&init, &[0, 1]), Err(ProtocolError::Magic));
    }

    #[test]
    fn each_encryption_uses_a_fresh_iv() {
        let k = [1u8; 16];
        assert_ne!(encrypt(&k, b"same"), encrypt(&k, b"same"));
        assert_eq!(decrypt(&k, &encrypt(&k, b"hello")).unwrap(), b"hello");
    }

    #[test]
    fn legacy_servers_send_no_nonces() {
        let mut init = sample_init();
        init.legacy = true;
        let wire = server_datagram(&init, &Packet::AuthenticateAck([9; 16]));
        assert_eq!(read_server_datagram(&init, &wire).unwrap(), Packet::AuthenticateAck([0; 16]));
        let check = client_datagram(&init, &Packet::ConnectionCheck([9; 16]));
        let modern = client_datagram(&sample_init(), &Packet::ConnectionCheck([9; 16]));
        assert_eq!(modern.len() - check.len(), 16);
    }

    #[test]
    fn legacy_authentication_carries_the_player_and_secret() {
        let mut init = sample_init();
        init.legacy = true;
        let wire = client_datagram(&init, &Packet::Authenticate);
        let mut r = Reader(&wire);
        assert_eq!(r.u8().unwrap(), MAGIC);
        assert_eq!(r.uuid().unwrap(), init.connection_id);
        let plain = decrypt(&init.secret, &r.bytes(MAX_PACKET).unwrap()).unwrap();
        assert_eq!(plain.len(), 1 + 16 + 16);
        assert_eq!(&plain[17..], &init.secret);
    }
}
