use mavlink::error::MessageReadError;
#[cfg(feature = "std")]
use std::env;
#[cfg(feature = "std")]
use std::sync::Arc;
#[cfg(feature = "std")]
use std::thread;
#[cfg(feature = "std")]
use std::time::Duration;

#[cfg(not(feature = "std"))]
fn main() {}

#[cfg(feature = "std")]
fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!(
            "Usage: mavlink-dump (tcpout|tcpin|udpout|udpin|udpbcast|serial|file):(ip|dev|path):(port|baud)"
        );
        return;
    }

    // It's possible to change the mavlink dialect to be used in the connect call
    let mut mavconn =
        mavlink::connect::<mavlink::mavlink::ardupilotmega::MavMessage>(&args[1]).unwrap();

    // here as an example we force the protocol version to mavlink V1:
    // the default for this library is mavlink V2
    mavconn.set_protocol_version(mavlink::MavlinkVersion::V1);

    let vehicle = Arc::new(mavconn);
    vehicle
        .send(&mavlink::MavHeader::default(), &request_parameters().into())
        .unwrap();
    vehicle
        .send(&mavlink::MavHeader::default(), &request_stream().into())
        .unwrap();

    thread::spawn({
        let vehicle = vehicle.clone();
        move || loop {
            let res = vehicle.send_default(&heartbeat_message().into());
            if res.is_ok() {
                thread::sleep(Duration::from_secs(1));
            } else {
                println!("send failed: {:?}", res);
            }
        }
    });

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
pub fn heartbeat_message() -> mavlink::mavlink::common::MavMessage {
    mavlink::mavlink::common::MavMessage::Heartbeat(mavlink::mavlink::common::Heartbeat {
        custom_mode: 0,
        mavtype: mavlink::mavlink::common::MavType::Quadrotor,
        autopilot: mavlink::mavlink::common::MavAutopilot::Ardupilotmega,
        base_mode: mavlink::mavlink::common::MavModeFlag::empty(),
        system_status: mavlink::mavlink::common::MavState::Standby,
        mavlink_version: 0x3,
    })
}

/// Create a message requesting the parameters list
#[cfg(feature = "std")]
pub fn request_parameters() -> mavlink::mavlink::common::MavMessage {
    mavlink::mavlink::common::MavMessage::ParamRequestList(
        mavlink::mavlink::common::ParamRequestList {
            target_system: 0,
            target_component: 0,
        },
    )
}

/// Create a message enabling data streaming
#[cfg(feature = "std")]
pub fn request_stream() -> mavlink::mavlink::common::MavMessage {
    mavlink::mavlink::common::MavMessage::RequestDataStream(
        mavlink::mavlink::common::RequestDataStream {
            target_system: 0,
            target_component: 0,
            req_stream_id: 0,
            req_message_rate: 10,
            start_stop: 1,
        },
    )
}
