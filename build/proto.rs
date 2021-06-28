use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::u32;

use crate::parser::*;
use crate::util::to_module_name;

impl MavProfile {
    pub fn emit_proto(
        &self,
        outf: &mut dyn Write,
        profile: &MavProfile,
        modules: &mut HashMap<String, MavProfile>,
    ) -> io::Result<()> {
        for inc in &self.includes {
            let inc_name = to_module_name(&inc);
            let mut inc_proto = PathBuf::from(&inc_name);
            inc_proto.set_extension("proto");
            writeln!(outf, "import \"{}\";", inc_proto.to_string_lossy())?;
        }
        for e in &self.enums {
            writeln!(outf)?;
            e.emit_proto(outf)?;
        }
        for message in &self.messages {
            writeln!(outf)?;
            message.emit_proto(outf, profile, modules)?;
        }
        Ok(())
    }
}

impl MavEnum {
    fn emit_proto(&self, outf: &mut dyn Write) -> io::Result<()> {
        writeln!(outf, "enum {} {{", self.raw_name)?;
        if let Some(description) = &self.description {
            for d in description.split('\n') {
                writeln!(outf, "// {}", d.trim())?;
            }
        }
        let mut sorted = self.entries.clone();
        sorted.sort_by(|a, b| {
            if a.value.is_none() && b.value.is_none() {
                return std::cmp::Ordering::Equal;
            }
            if a.value.is_none() {
                return std::cmp::Ordering::Greater;
            }
            if b.value.is_none() {
                return std::cmp::Ordering::Less;
            }
            if let (Some(a), Some(b)) = (a.value, b.value) {
                a.cmp(&b)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        // In case we have an enum with a missing value.
        let mut max_val: u32 = 0;
        let mut has_zero = false;
        for f in &sorted {
            if let Some(a) = f.value {
                if a == 0 {
                    has_zero = true;
                }
                if a > max_val {
                    max_val = a;
                }
            }
        }
        for (i, field) in sorted.iter().enumerate() {
            if i == 0 && !has_zero && max_val != 0 {
                // Do not have a 0 based enum field but protbuf requires it.
                writeln!(
                    outf,
                    "  // Not used in MavLink, make protobuf happy.\n  {}_UNDEFINED = 0;",
                    self.raw_name
                )?;
            }
            if let Some(description) = &field.description {
                for d in description.split('\n') {
                    writeln!(outf, "  // {}", d)?;
                }
            }
            writeln!(
                outf,
                "  {} = {};",
                field.raw_name,
                field.value.unwrap_or(max_val + i as u32)
            )?;
            if let Some(params) = &field.params {
                writeln!(outf, "  // ***** START Params")?;
                for p in params {
                    writeln!(outf, "  // {}", p)?;
                }
                writeln!(outf, "  // ***** END Params")?;
            }
        }
        writeln!(outf, "}}")?;
        Ok(())
    }
}

impl MavMessage {
    fn emit_proto(
        &self,
        outf: &mut dyn Write,
        profile: &MavProfile,
        modules: &mut HashMap<String, MavProfile>,
    ) -> io::Result<()> {
        if let Some(description) = &self.description {
            for d in description.split('\n') {
                writeln!(outf, "// {}", d.trim())?;
            }
        }
        writeln!(
            outf,
            "message {} {{  // MavLink id: {}",
            self.raw_name, self.id
        )?;
        for (i, field) in self.fields.iter().enumerate() {
            field.emit_proto(outf, i + 1, profile, modules)?;
        }
        writeln!(outf, "}}")?;
        Ok(())
    }
}

impl MavField {
    fn emit_proto(
        &self,
        outf: &mut dyn Write,
        id: usize,
        profile: &MavProfile,
        modules: &mut HashMap<String, MavProfile>,
    ) -> io::Result<()> {
        fn has_enum(enums: &[MavEnum], name: &str) -> bool {
            for e in enums {
                if e.name == name {
                    return true;
                }
            }
            false
        }
        if let Some(description) = &self.description {
            for d in description.split('\n') {
                writeln!(outf, "  // {}", d.trim())?;
            }
        }
        if let Some(enum_type) = &self.enumtype {
            let raw_type = self.raw_enumtype.as_ref().unwrap();
            // Got an enum, figure out if it is our enum or from an import.
            if has_enum(&profile.enums, enum_type) {
                writeln!(outf, "  {} {} = {};", raw_type, self.raw_name, id)?;
            } else {
                let mut found = false;
                for inc in &profile.includes {
                    let p = modules.get(inc).unwrap();
                    if has_enum(&p.enums, enum_type) {
                        found = true;
                        let inc_mod = to_module_name(&inc);
                        writeln!(
                            outf,
                            "  {}.{} {} = {};",
                            inc_mod, raw_type, self.raw_name, id
                        )?;
                        break;
                    }
                }
                if !found {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to find enum {}", enum_type),
                    ));
                }
            }
        } else {
            writeln!(
                outf,
                "  {} {} = {};",
                self.mavtype.proto_type(),
                self.raw_name,
                id
            )?;
        }
        Ok(())
    }
}

impl MavType {
    /// Return protobuf equivalent of a given Mavtype
    /// Used for generating proto message fields.
    fn proto_type(&self) -> String {
        use self::MavType::*;
        // XXX protobuf seems to not have anything less then 32 bits...
        match self.clone() {
            UInt8 | UInt8MavlinkVersion => "uint32".into(),
            Int8 => "uint32".into(),
            Char => "uint32".into(), // XXX should this be string?
            UInt16 => "uint32".into(),
            Int16 => "int32".into(),
            UInt32 => "uint32".into(),
            Int32 => "int32".into(),
            Float => "float".into(),
            UInt64 => "uint64".into(),
            Int64 => "int64".into(),
            Double => "double".into(),
            Array(t, _) => {
                if let MavType::Char = *t {
                    "string".into()
                } else {
                    format!("repeated {}", t.proto_type())
                    //"bytes".into()
                }
            }
        }
    }
}
