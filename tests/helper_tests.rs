#[cfg(test)]
#[cfg(all(feature = "std", feature = "common"))]
mod helper_tests {
    use proto_mav::mavlink::common::MavMessage;
    use proto_mav::Message;

    #[test]
    fn test_get_default_message_from_id() {
        let message_name = "Ping";
        let id: std::result::Result<u32, &'static str> =
            MavMessage::message_id_from_name(message_name);
        let id = id.unwrap();
        assert!(id == 4, "Invalid id for message name: Ping");
        let message = MavMessage::default_message_from_id(id);
        match message {
            Ok(MavMessage::Ping(_)) => {}
            _ => unreachable!("Invalid message type."),
        }
        assert!(
            message.unwrap().message_name() == message_name,
            "Message name does not match"
        );
    }
}
