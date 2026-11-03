#[cfg(test)]
mod tests {
    use crate::handshake::{create_handshake, decode_handshake_protocol};
    use crate::version::{MinecraftVersion, TargetVersion};

    #[test]
    fn mock_26_2_server_receives_handshake() {
        let target = TargetVersion::from_protocol(776);
        let handshake_bytes = create_handshake(&target, "localhost", 25565, false).encode();

        let protocol = decode_handshake_protocol(&handshake_bytes).unwrap();
        assert_eq!(protocol, 776);

        let detected = TargetVersion::from_protocol(protocol);
        assert_eq!(detected.minecraft_version(), MinecraftVersion::V26_2);
        assert!(detected.requires_translation());
    }

    #[test]
    fn native_handshake_uses_777() {
        let target = TargetVersion::from_protocol(777);
        let handshake = create_handshake(&target, "localhost", 25565, false);
        let encoded = handshake.encode();

        let protocol = decode_handshake_protocol(&encoded).unwrap();
        assert_eq!(protocol, 777);

        let detected = TargetVersion::from_protocol(protocol);
        assert_eq!(detected.minecraft_version(), MinecraftVersion::V26_3);
        assert!(!detected.requires_translation());
    }

    #[test]
    fn different_versions_produce_different_handshakes() {
        let h_26_3 = create_handshake(
            &TargetVersion::from_protocol(777),
            "localhost",
            25565,
            false,
        )
        .encode();
        let h_26_2 = create_handshake(
            &TargetVersion::from_protocol(776),
            "localhost",
            25565,
            false,
        )
        .encode();
        let h_26_1 = create_handshake(
            &TargetVersion::from_protocol(775),
            "localhost",
            25565,
            false,
        )
        .encode();

        assert_ne!(h_26_3, h_26_2);
        assert_ne!(h_26_2, h_26_1);
        assert_ne!(h_26_3, h_26_1);
    }

    #[test]
    fn version_detection_from_protocol_number() {
        for (protocol, expected_mc) in [(777, "26.3"), (776, "26.2"), (775, "26.1")] {
            let target = TargetVersion::from_protocol(protocol);
            assert_eq!(target.minecraft_version().as_str(), expected_mc);
        }
    }

    #[test]
    fn unsupported_protocol_rejected() {
        let target = TargetVersion::from_protocol(123);
        assert!(!target.is_supported());
        assert!(target.validate().is_err());
    }

    #[test]
    fn transferred_handshake_uses_target_protocol() {
        let target = TargetVersion::from_protocol(776);
        let handshake = create_handshake(&target, "localhost", 25565, true);
        let encoded = handshake.encode();

        let protocol = decode_handshake_protocol(&encoded).unwrap();
        assert_eq!(protocol, 776);

        match handshake {
            horizon_protocol::packets::handshake::Serverbound::Intention(intention) => {
                assert_eq!(
                    intention.intent,
                    horizon_protocol::packets::handshake::TRANSFER
                );
            }
            _ => panic!("expected Intention packet"),
        }
    }

    #[test]
    fn handshake_host_and_port_preserved() {
        let target = TargetVersion::from_protocol(776);
        let handshake = create_handshake(&target, "example.com", 25566, false);

        match handshake {
            horizon_protocol::packets::handshake::Serverbound::Intention(intention) => {
                assert_eq!(intention.host, "example.com");
                assert_eq!(intention.port, 25566);
                assert_eq!(intention.protocol, 776);
            }
            _ => panic!("expected Intention packet"),
        }
    }

    #[test]
    fn roundtrip_all_versions() {
        for protocol in [777, 776, 775] {
            let target = TargetVersion::from_protocol(protocol);
            let encoded = create_handshake(&target, "localhost", 25565, false).encode();
            let decoded_protocol = decode_handshake_protocol(&encoded).unwrap();
            assert_eq!(decoded_protocol, protocol);
        }
    }

    #[test]
    fn decode_invalid_handshake_fails() {
        let result = decode_handshake_protocol(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }
}
