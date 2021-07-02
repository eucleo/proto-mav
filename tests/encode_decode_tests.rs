mod test_shared;

#[cfg(test)]
#[cfg(feature = "common")]
mod test_encode_decode {
    use ::mavlink::*;

    #[test]
    pub fn test_echo_heartbeat() {
        let mut v = vec![];
        let send_msg = crate::test_shared::get_heartbeat_msg();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &mavlink::common::MavMessage::Heartbeat(send_msg.clone()),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg): (MavHeader, mavlink::common::MavMessage) =
            read_v2_msg(&mut c).expect("Failed to read");
        assert_eq!(recv_msg.message_id(), 0);
    }

    #[test]
    pub fn test_echo_command_int() {
        let mut v = vec![];
        let send_msg = crate::test_shared::get_cmd_nav_takeoff_msg();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &mavlink::common::MavMessage::CommandInt(send_msg.clone()),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg) = read_v2_msg(&mut c).expect("Failed to read");

        if let mavlink::common::MavMessage::CommandInt(recv_msg) = recv_msg {
            assert_eq!(recv_msg.command, proto::common::MavCmd::NavTakeoff as i32);
        } else {
            panic!("Decoded wrong message type")
        }
    }

    #[test]
    pub fn test_echo_hil_actuator_controls() {
        let mut v = vec![];
        let send_msg = crate::test_shared::get_hil_actuator_controls_msg();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &mavlink::common::MavMessage::HilActuatorControls(send_msg.clone()),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg) = read_v2_msg(&mut c).expect("Failed to read");
        if let mavlink::common::MavMessage::HilActuatorControls(recv_msg) = recv_msg {
            assert_eq!(
                proto::common::MavModeFlag::CustomModeEnabled as i32,
                recv_msg.mode & proto::common::MavModeFlag::CustomModeEnabled as i32
            );
        } else {
            panic!("Decoded wrong message type")
        }
    }

    /// This test makes sure that we can still receive messages in the common set
    /// properly when we're trying to decode APM messages.
    #[test]
    #[cfg(all(feature = "ardupilotmega", feature = "uavionix", feature = "icarous"))]
    pub fn test_echo_apm_heartbeat() {
        use self::mavlink::ardupilotmega::MavMessage;

        let mut v = vec![];
        let send_msg = crate::test_shared::get_heartbeat_msg();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &MavMessage::Common(mavlink::common::MavMessage::Heartbeat(send_msg.clone())),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg) = read_v2_msg(&mut c).expect("Failed to read");

        if let MavMessage::Common(recv_msg) = recv_msg {
            match &recv_msg {
                mavlink::common::MavMessage::Heartbeat(_data) => {
                    assert_eq!(recv_msg.message_id(), 0);
                }
                _ => panic!("Decoded wrong message type"),
            }
        } else {
            panic!("Decoded wrong message type")
        }
    }

    /// This test makes sure that messages that are not
    /// in the common set also get encoded and decoded
    /// properly.
    #[test]
    #[cfg(all(feature = "ardupilotmega", feature = "uavionix", feature = "icarous"))]
    pub fn test_echo_apm_mount_status() {
        use self::mavlink::ardupilotmega::MavMessage;

        let mut v = vec![];
        let send_msg = crate::test_shared::get_apm_mount_status();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &MavMessage::MountStatus(send_msg.clone()),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg) = read_v2_msg(&mut c).expect("Failed to read");
        if let MavMessage::MountStatus(recv_msg) = recv_msg {
            assert_eq!(4, recv_msg.pointing_b);
        } else {
            panic!("Decoded wrong message type")
        }
    }

    #[test]
    #[cfg(all(feature = "ardupilotmega", feature = "uavionix", feature = "icarous"))]
    pub fn test_echo_apm_command_int() {
        use self::mavlink::ardupilotmega::MavMessage;

        let mut v = vec![];
        let send_msg = crate::test_shared::get_cmd_nav_takeoff_msg();

        write_v2_msg(
            &mut v,
            crate::test_shared::COMMON_MSG_HEADER,
            &MavMessage::Common(mavlink::common::MavMessage::CommandInt(send_msg.clone())),
        )
        .expect("Failed to write message");

        let mut c = v.as_slice();
        let (_header, recv_msg) = read_v2_msg(&mut c).expect("Failed to read");

        if let MavMessage::Common(recv_msg) = recv_msg {
            match &recv_msg {
                mavlink::common::MavMessage::CommandInt(data) => {
                    assert_eq!(data.command, proto::common::MavCmd::NavTakeoff as i32);
                }
                _ => panic!("Decoded wrong message type"),
            }
        } else {
            panic!("Decoded wrong message type")
        }
    }
}
