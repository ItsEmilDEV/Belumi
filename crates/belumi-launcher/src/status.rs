use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct ServerStatus {
    pub online: u32,
    pub max: u32,
    pub version: String,
    pub motd: String,
    pub latency_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Probe {
    Unknown,
    Checking,
    Up(ServerStatus),
    Down,
}

fn put_varint(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn read_varint(stream: &mut impl Read) -> std::io::Result<u32> {
    let mut value = 0u32;
    for i in 0..5 {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte)?;
        value |= u32::from(byte[0] & 0x7F) << (7 * i);
        if byte[0] & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "varint too long"))
}

fn packet(id: u8, body: &[u8]) -> Vec<u8> {
    let mut inner = vec![id];
    inner.extend_from_slice(body);
    let mut out = Vec::new();
    put_varint(&mut out, inner.len() as u32);
    out.extend_from_slice(&inner);
    out
}

/// `host`, `host:port` or `[v6]:port`, with the default port 25565.
pub fn split_address(address: &str) -> (String, u16) {
    let address = address.trim();
    if let Some(rest) = address.strip_prefix('[')
        && let Some((host, tail)) = rest.split_once(']')
    {
        let port = tail.strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(25565);
        return (host.to_owned(), port);
    }
    match address.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => match port.parse() {
            Ok(p) => (host.to_owned(), p),
            Err(_) => (address.to_owned(), 25565),
        },
        _ => (address.to_owned(), 25565),
    }
}

fn plain_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(o) => {
            let mut text = o.get("text").and_then(Value::as_str).unwrap_or("").to_owned();
            if let Some(extra) = o.get("extra").and_then(Value::as_array) {
                for part in extra {
                    text.push_str(&plain_text(part));
                }
            }
            text
        }
        Value::Array(parts) => parts.iter().map(plain_text).collect(),
        _ => String::new(),
    }
}

fn clean(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else if c == '\n' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out.trim().to_owned()
}

pub fn parse_status(json: &str, latency_ms: u32) -> Option<ServerStatus> {
    let v: Value = serde_json::from_str(json).ok()?;
    Some(ServerStatus {
        online: v["players"]["online"].as_u64().unwrap_or(0) as u32,
        max: v["players"]["max"].as_u64().unwrap_or(0) as u32,
        version: v["version"]["name"].as_str().unwrap_or("").to_owned(),
        motd: clean(&plain_text(&v["description"])),
        latency_ms,
    })
}

/// Server List Ping: the status and the time the ping packet took to come back.
pub fn ping(address: &str, timeout: Duration) -> Option<ServerStatus> {
    let (host, port) = split_address(address);
    let target = (host.as_str(), port).to_socket_addrs().ok()?.next()?;
    let mut stream = TcpStream::connect_timeout(&target, timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let mut handshake = Vec::new();
    put_varint(&mut handshake, 767);
    put_varint(&mut handshake, host.len() as u32);
    handshake.extend_from_slice(host.as_bytes());
    handshake.extend_from_slice(&port.to_be_bytes());
    put_varint(&mut handshake, 1);
    stream.write_all(&packet(0x00, &handshake)).ok()?;
    stream.write_all(&packet(0x00, &[])).ok()?;
    let _length = read_varint(&mut stream).ok()?;
    let id = read_varint(&mut stream).ok()?;
    if id != 0 {
        return None;
    }
    let size = read_varint(&mut stream).ok()? as usize;
    if size > 1 << 20 {
        return None;
    }
    let mut json = vec![0u8; size];
    stream.read_exact(&mut json).ok()?;
    let started = Instant::now();
    stream.write_all(&packet(0x01, &0x1234_5678i64.to_be_bytes())).ok()?;
    let latency = match (read_varint(&mut stream), read_varint(&mut stream)) {
        (Ok(_), Ok(1)) => started.elapsed().as_millis().min(9999) as u32,
        _ => 0,
    };
    parse_status(std::str::from_utf8(&json).ok()?, latency)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn addresses_split_into_host_and_port() {
        assert_eq!(split_address("play.example.net"), ("play.example.net".into(), 25565));
        assert_eq!(split_address("play.example.net:25570"), ("play.example.net".into(), 25570));
        assert_eq!(split_address(" 127.0.0.1:1 "), ("127.0.0.1".into(), 1));
        assert_eq!(split_address("[::1]:2000"), ("::1".into(), 2000));
        assert_eq!(split_address("[::1]"), ("::1".into(), 25565));
        assert_eq!(split_address("host:notaport"), ("host:notaport".into(), 25565));
    }

    #[test]
    fn a_status_reply_gives_players_version_and_a_plain_motd() {
        let json = r#"{"version":{"name":"Paper 1.21","protocol":767},"players":{"online":3,"max":20},"description":{"text":"§aHello ","extra":[{"text":"world"}]}}"#;
        let s = parse_status(json, 12).unwrap();
        assert_eq!((s.online, s.max, s.version.as_str(), s.motd.as_str(), s.latency_ms), (3, 20, "Paper 1.21", "Hello world", 12));
        let plain = parse_status(r#"{"description":"A\nB","players":{"online":0,"max":5}}"#, 0).unwrap();
        assert_eq!(plain.motd, "A B");
        assert!(parse_status("nope", 0).is_none());
    }

    #[test]
    fn a_fake_server_is_pinged_end_to_end() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            for _ in 0..2 {
                let len = read_varint(&mut s).unwrap();
                let mut body = vec![0u8; len as usize];
                s.read_exact(&mut body).unwrap();
            }
            let json = br#"{"version":{"name":"Test"},"players":{"online":1,"max":2},"description":"Hi"}"#;
            let mut body = Vec::new();
            put_varint(&mut body, json.len() as u32);
            body.extend_from_slice(json);
            s.write_all(&packet(0x00, &body)).unwrap();
            let len = read_varint(&mut s).unwrap();
            let mut ping = vec![0u8; len as usize];
            s.read_exact(&mut ping).unwrap();
            s.write_all(&packet(0x01, &ping[1..])).unwrap();
        });
        let status = ping(&format!("127.0.0.1:{port}"), Duration::from_secs(3)).unwrap();
        assert_eq!((status.online, status.max, status.version.as_str(), status.motd.as_str()), (1, 2, "Test", "Hi"));
    }

    #[test]
    fn a_closed_port_is_down() {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert!(ping(&format!("127.0.0.1:{port}"), Duration::from_millis(500)).is_none());
    }
}
