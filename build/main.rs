#![recursion_limit = "256"]
#[macro_use]
extern crate quote;

extern crate xml;

mod binder;
mod mavlink;
mod parser;
mod proto;
mod util;

use crate::util::to_module_name;
use std::collections::HashMap;
use std::env;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn main() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // find & apply patches to XML definitions to avoid crashes
    let mut patch_dir = src_dir.to_path_buf();
    patch_dir.push("build/patches");
    let mut mavlink_dir = src_dir.to_path_buf();
    mavlink_dir.push("mavlink");

    if let Ok(dir) = read_dir(patch_dir) {
        for entry in dir.flatten() {
            match Command::new("git")
                .arg("apply")
                .arg(entry.path().as_os_str())
                .current_dir(&mavlink_dir)
                .status()
            {
                Ok(_) => (),
                Err(error) => eprintln!("{}", error),
            }
        }
    }

    let mut definitions_dir = src_dir.to_path_buf();
    definitions_dir.push("mavlink/message_definitions/v1.0");

    let out_dir = format!("{}/proto-mav-gen", src_dir.display());
    let mav_out = format!("{}/proto-mav-gen/src/mavlink", src_dir.display());
    if std::fs::create_dir_all(&mav_out).is_err() {} // Do not care if this exists.
    let proto_out = format!("{}/proto-mav-gen/src/proto", src_dir.display());
    if std::fs::create_dir(&proto_out).is_err() {} // Do not care if this exists.
    let protobufs_out = format!("{}/proto-mav-gen/protos", src_dir.display());
    if std::fs::create_dir(&protobufs_out).is_err() {} // Do not care if this exists.

    let mut modules = vec![];
    let mut modules_map: HashMap<String, parser::MavProfile> = HashMap::new();

    for entry in read_dir(&definitions_dir).expect("could not read definitions directory") {
        let entry = entry.expect("could not read directory entry");

        let definition_file = entry.file_name();
        let module_name = to_module_name(&definition_file);

        modules.push(module_name);

        parser::generate(
            &definitions_dir,
            &definition_file,
            &out_dir,
            &mut modules_map,
        );
    }

    // output mod.rs for src
    {
        let out_dir = Path::new(&out_dir).join("src");
        let dest_path = Path::new(&out_dir).join("lib.rs");
        {
            let mut outf = File::create(&dest_path).unwrap();

            let src_modules = vec!["mavlink".to_string(), "proto".to_string()];
            // generate code
            binder::generate_bare(&src_modules, &mut outf);
        }

        // format code
        match Command::new("rustfmt")
            .arg(dest_path.as_os_str())
            .current_dir(&out_dir)
            .status()
        {
            Ok(_) => (),
            Err(error) => eprintln!("{}", error),
        }
    }

    // output mod.rs for mavlink
    {
        let out_dir = Path::new(&out_dir).join("src").join("mavlink");
        let dest_path = Path::new(&out_dir).join("mod.rs");
        {
            let mut outf = File::create(&dest_path).unwrap();

            // generate code
            binder::generate(&modules, &mut outf);
        }

        // format code
        match Command::new("rustfmt")
            .arg(dest_path.as_os_str())
            .current_dir(&out_dir)
            .status()
        {
            Ok(_) => (),
            Err(error) => eprintln!("{}", error),
        }
    }

    {
        let dest_path = Path::new(&protobufs_out).join("mav.proto");
        let mut outf = File::create(&dest_path).unwrap();
        let opts = r#"
syntax = "proto3";

package mav;

import "google/protobuf/descriptor.proto";

message MavFieldOptions {
  optional string type = 1;
  optional string enum = 2;
  optional string display = 3;
}

message MavMesOptions {
  optional int32 id = 1;
}

extend google.protobuf.FieldOptions {
  optional MavFieldOptions opts = 60066;
}
extend google.protobuf.MessageOptions {
  optional MavMesOptions message = 60066;
}
"#;
        outf.write_all(opts.as_bytes()).unwrap();
    }
    {
        let dest_path = Path::new(&out_dir).join("README.md");
        let mut outf = File::create(&dest_path).unwrap();
        let opts = r#"
This repo is autogenerated from git@github.com:eucleo/proto-mav.git
It exists to avoid a bunch of unnessarry code generation in projects that use it.
DO NOT edit this by hand.
"#;
        outf.write_all(opts.as_bytes()).unwrap();
    }
    {
        let dest_path = Path::new(&out_dir).join("Cargo.toml");
        let mut outf = File::create(&dest_path).unwrap();
        let opts = r#"
[package]
name = "proto_mav_gen"
version = "0.10.0"
description = "Code auto generated by git@github.com:eucleo/proto-mav.git DO NOT EDIT."
readme = "README.md"
license = "MIT/Apache-2.0"
repository = "https://github.com/eucleo/proto-mav-gen"
edition = "2018"

[dependencies]
bytes = { version = "1.0", default-features = false }
num-traits = { version = "0.2", default-features = false }
num-derive = "0.3.2"
bitflags = "1.2.1"
proto_mav_comm = { git="https://github.com/eucleo/proto-mav-comm.git" }
serde = { version = "1" }
prost = "0.9"
"#;
        outf.write_all(opts.as_bytes()).unwrap();
    }
    let mut protos = Vec::new();
    for module in &modules {
        protos.push(format!("{}/{}.proto", protobufs_out, module));
    }
    prost_build::Config::new()
        .out_dir(proto_out)
        //        .compile_well_known_types()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&protos, &[protobufs_out])
        .unwrap();

    // output mod.rs for proto
    {
        let out_dir = Path::new(&out_dir).join("src").join("proto");
        let dest_path = Path::new(&out_dir).join("mod.rs");
        {
            let mut outf = File::create(&dest_path).unwrap();

            // generate code
            binder::generate(&modules, &mut outf);
        }

        // format code
        match Command::new("rustfmt")
            .arg(dest_path.as_os_str())
            .current_dir(&out_dir)
            .status()
        {
            Ok(_) => (),
            Err(error) => eprintln!("{}", error),
        }
    }
}
