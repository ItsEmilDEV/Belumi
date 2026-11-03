use crate::version::TargetVersion;
use horizon_protocol::packets::handshake;

pub fn create_handshake(
    target: &TargetVersion,
    host: &str,
    port: u16,
    transferred: bool,
) -> handshake::Serverbound {
    let intent = if transferred {
        handshake::TRANSFER
    } else {
        handshake::LOGIN
    };

    handshake::Serverbound::Intention(handshake::Intention {
        protocol: target.protocol_version().as_i32(),
        host: host.to_owned(),
        port,
        intent,
    })
}

pub fn encode_handshake(
    target: &TargetVersion,
    host: &str,
    port: u16,
    transferred: bool,
) -> Vec<u8> {
    create_handshake(target, host, port, transferred).encode()
}

pub fn decode_handshake_protocol(bytes: &[u8]) -> Result<i32, String> {
    let mut r = horizon_protocol::buf::Reader::new(bytes);
    let id = r.varint().map_err(|e| e.to_string())?;
    if id != 0 {
        return Err(format!("unexpected packet id: {}", id));
    }
    let protocol = r.varint().map_err(|e| e.to_string())?;
    Ok(protocol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_contains_target_protocol_version() {
        let target = TargetVersion::from_protocol(776);
        let packet = create_handshake(&target, "localhost", 25565, false);
        let encoded = packet.encode();

        let protocol = decode_handshake_protocol(&encoded).unwrap();
        assert_eq!(protocol, 776);
    }

    #[test]
    fn handshake_for_26_1() {
        let target = TargetVersion::from_protocol(775);
        let packet = create_handshake(&target, "localhost", 25565, false);
        let encoded = packet.encode();

        let protocol = decode_handshake_protocol(&encoded).unwrap();
        assert_eq!(protocol, 775);
    }

    #[test]
    fn handshake_for_native_26_3() {
        let target = TargetVersion::from_protocol(777);
        let packet = create_handshake(&target, "localhost", 25565, false);
        let encoded = packet.encode();

        let protocol = decode_handshake_protocol(&encoded).unwrap();
        assert_eq!(protocol, 777);
    }

    #[test]
    fn handshake_transferred_intent() {
        let target = TargetVersion::from_protocol(776);
        let packet = create_handshake(&target, "localhost", 25565, true);

        match packet {
            handshake::Serverbound::Intention(intention) => {
                assert_eq!(intention.protocol, 776);
                assert_eq!(intention.intent, handshake::TRANSFER);
            }
            _ => panic!("expected Intention packet"),
        }
    }

    #[test]
    fn handshake_login_intent() {
        let target = TargetVersion::from_protocol(776);
        let packet = create_handshake(&target, "localhost", 25565, false);

        match packet {
            handshake::Serverbound::Intention(intention) => {
                assert_eq!(intention.protocol, 776);
                assert_eq!(intention.intent, handshake::LOGIN);
            }
            _ => panic!("expected Intention packet"),
        }
    }

    #[test]
    fn decode_invalid_packet_fails() {
        let result = decode_handshake_protocol(&[0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn handshake_host_and_port_preserved() {
        let target = TargetVersion::from_protocol(776);
        let packet = create_handshake(&target, "example.com", 25566, false);

        match packet {
            handshake::Serverbound::Intention(intention) => {
                assert_eq!(intention.host, "example.com");
                assert_eq!(intention.port, 25566);
            }
            _ => panic!("expected Intention packet"),
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let target = TargetVersion::from_protocol(776);
        let encoded = encode_handshake(&target, "localhost", 25565, false);
        let protocol = decode_handshake_protocol(&encoded).unwrap();
        assert_eq!(protocol, 776);
    }
}
