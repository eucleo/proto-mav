mod test_shared;

#[cfg(test)]
#[cfg(all(feature = "std", feature = "tcp", feature = "common"))]
mod test_tcp_connections {
    use proto_mav::*;
    use std::thread;

    /// Test whether we can send a message via TCP and receive it OK
    #[test]
    pub fn test_tcp_loopback() {
        const RECEIVE_CHECK_COUNT: i32 = 5;

        let server_thread = thread::spawn(move || {
            //TODO consider using get_available_port to use a random port
            let server = connect("tcpin:0.0.0.0:14550").expect("Couldn't create server");

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
        });

        // Give some time for the server to connect
        thread::sleep(std::time::Duration::from_millis(100));

        // have the client send a few hearbeats
        thread::spawn(move || {
            let msg =
                mavlink::common::MavMessage::Heartbeat(crate::test_shared::get_heartbeat_msg());
            let client = connect("tcpout:127.0.0.1:14550").expect("Couldn't create client");
            for _i in 0..RECEIVE_CHECK_COUNT {
                client.send_default(&msg).ok();
            }
        });

        server_thread.join().unwrap();
    }
}
