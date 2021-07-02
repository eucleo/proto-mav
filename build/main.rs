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
use std::path::Path;
use std::process::Command;

pub fn main() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Update and init submodule
    match Command::new("git")
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .current_dir(&src_dir)
        .status()
    {
        Ok(_) => {}
        Err(error) => eprintln!("{}", error),
    }

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

    let out_dir = env::var("OUT_DIR").unwrap();
    let mav_out = format!("{}/src/mavlink", out_dir);
    if std::fs::create_dir_all(&mav_out).is_err() {} // Do not care if this exists.
    let proto_out = format!("{}/src/proto", out_dir);
    if std::fs::create_dir(&proto_out).is_err() {} // Do not care if this exists.

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
        let dest_path = Path::new(&out_dir).join("mod.rs");
        let mut outf = File::create(&dest_path).unwrap();

        let src_modules = vec!["mavlink".to_string(), "proto".to_string()];
        // generate code
        binder::generate_bare(&src_modules, &mut outf);

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
        let mut outf = File::create(&dest_path).unwrap();

        // generate code
        binder::generate(&modules, &mut outf);

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

    let mut protos = Vec::new();
    for module in &modules {
        protos.push(format!("{}/{}.proto", out_dir, module));
    }
    let proto_out = format!("{}/src/proto", out_dir);
    prost_build::Config::new()
        .out_dir(proto_out)
        //        .compile_well_known_types()
        .compile_protos(&protos, &[out_dir.clone()])
        .unwrap();

    // output mod.rs for proto
    {
        let out_dir = Path::new(&out_dir).join("src").join("proto");
        let dest_path = Path::new(&out_dir).join("mod.rs");
        let mut outf = File::create(&dest_path).unwrap();

        // generate code
        binder::generate(&modules, &mut outf);

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
