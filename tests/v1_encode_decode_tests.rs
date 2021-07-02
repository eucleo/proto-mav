pub mod test_shared;

#[cfg(test)]
#[cfg(all(feature = "std", feature = "common"))]
mod test_v1_encode_decode {
    use proto_mav::*;

    pub const HEARTBEAT_V1: &'static [u8] = &[
        MAV_STX, 0x09, 0xef, 0x01, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x02, 0x03, 0x59, 0x03,
        0x03, 0xf1, 0xd7,
    ];

    #[test]
    pub fn test_read_heartbeat() {
        let mut r = HEARTBEAT_V1;
        let (header, msg) = read_v1_msg(&mut r).expect("Failed to parse message");
        //println!("{:?}, {:?}", header, msg);

        assert_eq!(header, crate::test_shared::COMMON_MSG_HEADER);
        let heartbeat_msg = crate::test_shared::get_heartbeat_msg();

        if let mavlink::common::MavMessage::Heartbeat(msg) = msg {
            assert_eq!(msg.custom_mode, heartbeat_msg.custom_mode);
            assert_eq!(msg.r#type, heartbeat_msg.r#type);
            assert_eq!(msg.autopilot, heartbeat_msg.autopilot);
            assert_eq!(msg.base_mode, heartbeat_msg.base_mode);
            assert_eq!(msg.system_status, heartbeat_msg.system_status);
            assert_eq!(msg.mavlink_version, heartbeat_msg.mavlink_version);
        } else {
            panic!("Decoded wrong message type")
        }
    }

    #[test]
    pub fn test_write_heartbeat() {
        let mut v = vec![];
        let heartbeat_msg = crate::test_shared::get_heartbeat_msg();
        write_v1_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &mavlink::common::MavMessage::Heartbeat(heartbeat_msg.clone()),
        )
        .expect("Failed to write message");

        assert_eq!(&v[..], HEARTBEAT_V1);
    }

    #[test]
    #[cfg(not(feature = "emit-extensions"))]
    pub fn test_echo_servo_output_raw() {
        use proto_mav::Message;

        let mut v = vec![];
        let send_msg = crate::test_shared::get_servo_output_raw_v1();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &mavlink::common::MavMessage::ServoOutputRaw(send_msg.clone()),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg): (MavHeader, mavlink::common::MavMessage) =
            read_v2_msg(&mut c).expect("Failed to read");

        assert_eq!(
            mavlink::common::MavMessage::extra_crc(recv_msg.message_id()),
            222 as u8
        );

        if let mavlink::common::MavMessage::ServoOutputRaw(recv_msg) = recv_msg {
            assert_eq!(recv_msg.port, 123 as u32);
            assert_eq!(recv_msg.servo4_raw, 1400 as u32);
        } else {
            panic!("Decoded wrong message type")
        }
    }
}
