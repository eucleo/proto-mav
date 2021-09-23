use proto_mav::{error::MessageReadError, *};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    // It's possible to change the mavlink dialect to be used in the connect call
    let mut mavconn = connect::<mavlink::common::MavMessage>("tcpout:127.0.0.1:9993").unwrap();

    // here as an example we force the protocol version to mavlink V1:
    // the default for this library is mavlink V2
    mavconn.set_protocol_version(MavlinkVersion::V2);

    let vehicle = Arc::new(mavconn);
    vehicle
        .send(&MavHeader::default(), &request_parameters())
        .unwrap();
    vehicle
        .send(&MavHeader::default(), &request_stream())
        .unwrap();

    loop {
        match vehicle.recv() {
            Ok((_header, msg)) => {
                println!("received: {:?}", msg);
            }
            Err(MessageReadError::Io(e)) => {
                match e.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        //no messages currently available to receive -- wait a while
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    _ => {
                        println!("recv error: {:?}", e);
                        break;
                    }
                }
            }
            // messages that didn't get through due to parser errors are ignored
            _ => {}
        }
    }
}

/// Create a heartbeat message using ardupilotmega dialect
/// If only common dialect is used, the `ardupilotmega::MavMessage::common` is not necessary,
/// and the function could return only a simple `mavlink::common::MavMessage` type
#[cfg(feature = "std")]
pub fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::Heartbeat(proto::common::Heartbeat {
        custom_mode: 0,
        r#type: proto::common::MavType::Quadrotor as i32,
        autopilot: proto::common::MavAutopilot::Ardupilotmega as i32,
        base_mode: proto::common::MavModeFlag::Undefined as u32,
        system_status: proto::common::MavState::Standby as i32,
        mavlink_version: 0x3,
    })
}

/// Create a message requesting the parameters list
#[cfg(feature = "std")]
pub fn request_parameters() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::ParamRequestList(proto::common::ParamRequestList {
        target_system: 0,
        target_component: 0,
    })
}

/// Create a message enabling data streaming
#[cfg(feature = "std")]
pub fn request_stream() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::RequestDataStream(proto::common::RequestDataStream {
        target_system: 0,
        target_component: 0,
        req_stream_id: 0,
        req_message_rate: 10,
        start_stop: 1,
    })
}
