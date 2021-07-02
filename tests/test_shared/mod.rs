use proto_mav::*;

#[allow(dead_code)]
pub const COMMON_MSG_HEADER: MavHeader = MavHeader {
    sequence: 239,
    system_id: 1,
    component_id: 1,
};

#[cfg(feature = "common")]
pub fn get_heartbeat_msg() -> proto::common::Heartbeat {
    proto::common::Heartbeat {
        custom_mode: 5,
        r#type: proto::common::MavType::Quadrotor as i32,
        autopilot: proto::common::MavAutopilot::Ardupilotmega as i32,
        base_mode: proto::common::MavModeFlag::ManualInputEnabled as i32
            | proto::common::MavModeFlag::StabilizeEnabled as i32
            | proto::common::MavModeFlag::GuidedEnabled as i32
            | proto::common::MavModeFlag::CustomModeEnabled as i32,
        system_status: proto::common::MavState::Standby as i32,
        mavlink_version: 3,
    }
}

#[allow(dead_code)]
#[cfg(feature = "common")]
pub fn get_cmd_nav_takeoff_msg() -> proto::common::CommandInt {
    proto::common::CommandInt {
        param1: 1.0,
        param2: 2.0,
        param3: 3.0,
        param4: 4.0,
        x: 555,
        y: 666,
        z: 777.0,
        command: proto::common::MavCmd::NavTakeoff as i32,
        target_system: 42,
        target_component: 84,
        frame: proto::common::MavFrame::Global as i32,
        current: 73,
        autocontinue: 17,
    }
}

#[allow(dead_code)]
#[cfg(feature = "common")]
pub fn get_hil_actuator_controls_msg() -> proto::common::HilActuatorControls {
    proto::common::HilActuatorControls {
        time_usec: 1234567 as u64,
        flags: 0 as u64,
        controls: vec![
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        ],
        mode: proto::common::MavModeFlag::ManualInputEnabled as i32
            | proto::common::MavModeFlag::StabilizeEnabled as i32
            | proto::common::MavModeFlag::CustomModeEnabled as i32,
    }
}

#[allow(dead_code)]
#[cfg(all(feature = "common", not(feature = "emit-extensions")))]
pub fn get_servo_output_raw_v1() -> proto::common::ServoOutputRaw {
    proto::common::ServoOutputRaw {
        time_usec: 1234567 as u32,
        servo1_raw: 1100 as u32,
        servo2_raw: 1200 as u32,
        servo3_raw: 1300 as u32,
        servo4_raw: 1400 as u32,
        servo5_raw: 1500 as u32,
        servo6_raw: 1600 as u32,
        servo7_raw: 1700 as u32,
        servo8_raw: 1800 as u32,
        port: 123 as u32,
    }
}

#[allow(dead_code)]
#[cfg(all(feature = "common", feature = "emit-extensions"))]
pub fn get_servo_output_raw_v2() -> proto::common::ServoOutputRaw {
    proto::common::ServoOutputRaw {
        time_usec: 1234567 as u32,
        servo1_raw: 1100 as u32,
        servo2_raw: 1200 as u32,
        servo3_raw: 1300 as u32,
        servo4_raw: 1400 as u32,
        servo5_raw: 1500 as u32,
        servo6_raw: 1600 as u32,
        servo7_raw: 1700 as u32,
        servo8_raw: 1800 as u32,
        port: 123 as u32,
        servo9_raw: 1110 as u32,
        servo10_raw: 1220 as u32,
        servo11_raw: 1330 as u32,
        servo12_raw: 1440 as u32,
        servo13_raw: 1550 as u32,
        servo14_raw: 1660 as u32,
        servo15_raw: 1770 as u32,
        servo16_raw: 1880 as u32,
    }
}

#[allow(dead_code)]
#[cfg(all(feature = "ardupilotmega", feature = "uavionix", feature = "icarous"))]
pub fn get_apm_mount_status() -> proto::ardupilotmega::MountStatus {
    proto::ardupilotmega::MountStatus {
        pointing_a: 3,
        pointing_b: 4,
        pointing_c: 5,
        target_system: 2,
        target_component: 3,
    }
}
