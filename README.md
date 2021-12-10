# proto-mav

This is a HEAVILY hacked fork of https://github.com/mavlink/rust-mavlink
It generates protobuf proto files from the mavlink files, the rust structs to
go with them and then also generates code to (de)serialize the structs to/from
mavlink as well as protobuf (using the same base structs).

Use or see the update.sh script for how to use it.  It is used to generate the
proto-mav-gen repo to make it easier to use the code in other projects.

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

