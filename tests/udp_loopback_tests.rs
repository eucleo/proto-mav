mod test_shared;

#[cfg(test)]
#[cfg(all(feature = "std", feature = "udp", feature = "common"))]
mod test_udp_connections {
    use proto_mav::*;
    use std::thread;

    /// Test whether we can send a message via UDP and receive it OK
    #[test]
    pub fn test_udp_loopback() {
        const RECEIVE_CHECK_COUNT: i32 = 3;

        let server = connect("udpin:0.0.0.0:14551").expect("Couldn't create server");

        // have the client send one heartbeat per second
        thread::spawn({
            move || {
                let msg =
                    mavlink::common::MavMessage::Heartbeat(crate::test_shared::get_heartbeat_msg());
                let client = connect("udpout:127.0.0.1:14551").expect("Couldn't create client");
                loop {
                    client.send_default(&msg).ok();
                }
            }
        });

        //TODO use std::sync::WaitTimeoutResult to timeout ourselves if recv fails?
        let mut recv_count = 0;
        for _i in 0..RECEIVE_CHECK_COUNT {
            match server.recv() {
                Ok((_header, msg)) => {
                    match msg {
                        mavlink::common::MavMessage::Heartbeat(_heartbeat_msg) => {
                            recv_count += 1;
                        }
                        _ => {
                            // one message parse failure fails the test
                            break;
                        }
                    }
                }
                Err(..) => {
                    // one message read failure fails the test
                    break;
                }
            }
        }
        assert_eq!(recv_count, RECEIVE_CHECK_COUNT);
    }
}
