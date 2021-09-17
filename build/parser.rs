use std::cmp::Ordering;
use std::collections::HashMap;
use std::default::Default;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::u32;

use heck::{CamelCase, SnakeCase};
use xml::reader::{EventReader, XmlEvent};

use crate::util::to_module_name;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MavProfile {
    pub includes: Vec<String>,
    pub messages: Vec<MavMessage>,
    pub enums: Vec<MavEnum>,
}

impl MavProfile {
    /// Go over all fields in the messages, and if you encounter an enum,
    /// update this enum with information about whether it is a bitmask, and what
    /// is the desired width of such.
    fn update_enums(mut self) -> Self {
        for msg in &self.messages {
            for field in &msg.fields {
                if let Some(ref enum_name) = field.enumtype {
                    // it is an enum
                    if let Some(ref dsp) = field.display {
                        // it is a bitmask
                        if dsp == "bitmask" {
                            // find the corresponding enum
                            for mut enm in &mut self.enums {
                                if enm.name == *enum_name {
                                    // this is the right enum
                                    if enm.bitfield.is_none() {
                                        enm.bitfield = Some("u32".into());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self
    }

    //TODO verify this is no longer necessary since we're supporting both mavlink1 and mavlink2
    //    ///If we are not using Mavlink v2, remove messages with id's > 254
    //    fn update_messages(mut self) -> Self {
    //        //println!("Updating messages");
    //        let msgs = self.messages.into_iter().filter(
    //            |x| x.id <= 254).collect::<Vec<MavMessage>>();
    //        self.messages = msgs;
    //        self
    //    }
}

#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MavEnum {
    pub name: String,
    pub raw_name: String,
    pub description: Option<String>,
    pub entries: Vec<MavEnumEntry>,
    /// If contains Some, the string represents the type witdh for bitflags
    pub bitfield: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MavEnumEntry {
    pub value: Option<u32>,
    pub name: String,
    pub raw_name: String,
    pub description: Option<String>,
    pub params: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MavMessage {
    pub id: u32,
    pub name: String,
    pub raw_name: String,
    pub description: Option<String>,
    pub fields: Vec<MavField>,
}

#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MavField {
    pub mavtype: MavType,
    pub name: String,
    pub raw_name: String,
    pub description: Option<String>,
    pub enumtype: Option<String>,
    pub raw_enumtype: Option<String>,
    pub display: Option<String>,
    pub is_extension: bool,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MavType {
    UInt8MavlinkVersion,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Char,
    Float,
    Double,
    Array(Box<MavType>, usize),
}

impl Default for MavType {
    fn default() -> MavType {
        MavType::UInt8
    }
}

impl MavType {
    pub fn parse_type(s: &str) -> Option<MavType> {
        use self::MavType::*;
        match s {
            "uint8_t_mavlink_version" => Some(UInt8MavlinkVersion),
            "uint8_t" => Some(UInt8),
            "uint16_t" => Some(UInt16),
            "uint32_t" => Some(UInt32),
            "uint64_t" => Some(UInt64),
            "int8_t" => Some(Int8),
            "int16_t" => Some(Int16),
            "int32_t" => Some(Int32),
            "int64_t" => Some(Int64),
            "char" => Some(Char),
            "float" => Some(Float),
            "Double" => Some(Double),
            "double" => Some(Double),
            _ => {
                if s.ends_with(']') {
                    let start = s.find('[')?;
                    let size = s[start + 1..(s.len() - 1)].parse::<usize>().ok()?;
                    let mtype = MavType::parse_type(&s[0..start])?;
                    Some(Array(Box::new(mtype), size))
                } else {
                    None
                }
            }
        }
    }

    /// Size of a given Mavtype
    pub fn len(&self) -> usize {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion | UInt8 | Int8 | Char => 1,
            UInt16 | Int16 => 2,
            UInt32 | Int32 | Float => 4,
            UInt64 | Int64 | Double => 8,
            Array(t, size) => t.len() * size,
        }
    }

    /// Used for ordering of types
    pub fn order_len(&self) -> usize {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion | UInt8 | Int8 | Char => 1,
            UInt16 | Int16 => 2,
            UInt32 | Int32 | Float => 4,
            UInt64 | Int64 | Double => 8,
            Array(t, _) => t.len(),
        }
    }

    /// Used for crc calculation
    pub fn primitive_type(&self) -> String {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion => "uint8_t".into(),
            UInt8 => "uint8_t".into(),
            Int8 => "int8_t".into(),
            Char => "char".into(),
            UInt16 => "uint16_t".into(),
            Int16 => "int16_t".into(),
            UInt32 => "uint32_t".into(),
            Int32 => "int32_t".into(),
            Float => "float".into(),
            UInt64 => "uint64_t".into(),
            Int64 => "int64_t".into(),
            Double => "double".into(),
            Array(t, _) => t.primitive_type(),
        }
    }

    /// Used for proto annotations
    pub fn mav_type(&self) -> String {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion => "uint8_t".into(),
            UInt8 => "uint8_t".into(),
            Int8 => "int8_t".into(),
            Char => "char".into(),
            UInt16 => "uint16_t".into(),
            Int16 => "int16_t".into(),
            UInt32 => "uint32_t".into(),
            Int32 => "int32_t".into(),
            Float => "float".into(),
            UInt64 => "uint64_t".into(),
            Int64 => "int64_t".into(),
            Double => "double".into(),
            Array(t, s) => format!("{}[{}]", t.mav_type(), s),
        }
    }

    /// Return rust equivalent of a given Mavtype
    /// Used for generating struct fields.
    pub fn rust_type(&self) -> String {
        use self::MavType::*;
        match self.clone() {
            UInt8 | UInt8MavlinkVersion => "u8".into(),
            Int8 => "i8".into(),
            Char => "char".into(),
            UInt16 => "u16".into(),
            Int16 => "i16".into(),
            UInt32 => "u32".into(),
            Int32 => "i32".into(),
            Float => "f32".into(),
            UInt64 => "u64".into(),
            Int64 => "i64".into(),
            Double => "f64".into(),
            Array(t, size) => {
                if size > 32 {
                    // we have to use a vector to make our lives easier
                    format!("Vec<{}> /* {} elements */", t.rust_type(), size)
                } else {
                    // we can use a slice, as Rust derives lot of thinsg for slices <= 32 elements
                    format!("[{};{}]", t.rust_type(), size)
                }
            }
        }
    }

    /// Compare two MavTypes
    pub fn compare(&self, other: &Self) -> Ordering {
        let len = self.order_len();
        (-(len as isize)).cmp(&(-(other.order_len() as isize)))
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MavXmlElement {
    Version,
    Mavlink,
    Dialect,
    Include,
    Enums,
    Enum,
    Entry,
    Description,
    Param,
    Messages,
    Message,
    Field,
    Deprecated,
    Wip,
    Extensions,
}

fn identify_element(s: &str) -> Option<MavXmlElement> {
    use self::MavXmlElement::*;
    match s {
        "version" => Some(Version),
        "mavlink" => Some(Mavlink),
        "dialect" => Some(Dialect),
        "include" => Some(Include),
        "enums" => Some(Enums),
        "enum" => Some(Enum),
        "entry" => Some(Entry),
        "description" => Some(Description),
        "param" => Some(Param),
        "messages" => Some(Messages),
        "message" => Some(Message),
        "field" => Some(Field),
        "deprecated" => Some(Deprecated),
        "wip" => Some(Wip),
        "extensions" => Some(Extensions),
        _ => None,
    }
}

fn is_valid_parent(p: Option<MavXmlElement>, s: MavXmlElement) -> bool {
    use self::MavXmlElement::*;
    match s {
        Version => p == Some(Mavlink),
        Mavlink => p == None,
        Dialect => p == Some(Mavlink),
        Include => p == Some(Mavlink),
        Enums => p == Some(Mavlink),
        Enum => p == Some(Enums),
        Entry => p == Some(Enum),
        Description => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Param => p == Some(Entry),
        Messages => p == Some(Mavlink),
        Message => p == Some(Messages),
        Field => p == Some(Message),
        Deprecated => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Wip => p == Some(Entry) || p == Some(Message) || p == Some(Enum),
        Extensions => p == Some(Message),
    }
}

pub fn snake_name(name: &str) -> String {
    let mut ident = name.to_snake_case();

    // Use a raw identifier if the identifier matches a Rust keyword:
    // https://doc.rust-lang.org/reference/keywords.html.
    match ident.as_str() {
        // 2015 strict keywords.
        | "as" | "break" | "const" | "continue" | "else" | "enum" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut"
        | "pub" | "ref" | "return" | "static" | "struct" | "trait" | "true"
        | "type" | "unsafe" | "use" | "where" | "while"
        // 2018 strict keywords.
        | "dyn"
        // 2015 reserved keywords.
        | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv" | "typeof"
        | "unsized" | "virtual" | "yield"
        // 2018 reserved keywords.
        | "async" | "await" | "try" => ident.insert_str(0, "r#"),
        // the following keywords are not supported as raw identifiers and are therefore suffixed with an underscore.
        "self" | "super" | "extern" | "crate" => ident += "_",
        _ => (),
    }
    ident
}

pub fn rusty_name(name: &str) -> String {
    let mut ident = name.to_camel_case();

    // Suffix an underscore for the `Self` Rust keyword as it is not allowed as raw identifier.
    if ident == "Self" {
        ident += "_";
    }
    ident
}

pub fn parse_profile(file: &mut dyn Read) -> MavProfile {
    let mut stack: Vec<MavXmlElement> = vec![];

    let mut profile = MavProfile {
        includes: vec![],
        messages: vec![],
        enums: vec![],
    };

    let mut field = MavField::default();
    let mut message = MavMessage::default();
    let mut mavenum = MavEnum::default();
    let mut entry = MavEnumEntry::default();
    let mut include = String::new();
    let mut paramid: Option<usize> = None;

    let mut xml_filter = MavXmlFilter::default();
    let mut parser: Vec<Result<XmlEvent, xml::reader::Error>> =
        EventReader::new(file).into_iter().collect();
    xml_filter.filter(&mut parser);
    let mut is_in_extension = false;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name,
                attributes: attrs,
                ..
            }) => {
                let id = match identify_element(&name.to_string()) {
                    None => {
                        panic!("unexpected element {:?}", name);
                    }
                    Some(kind) => kind,
                };

                //
                if !is_valid_parent(stack.last().copied(), id) {
                    panic!("not valid parent {:?} of {:?}", stack.last(), id);
                }

                match id {
                    MavXmlElement::Extensions => {
                        is_in_extension = true;
                    }
                    MavXmlElement::Message => {
                        message = Default::default();
                    }
                    MavXmlElement::Field => {
                        field = Default::default();
                        field.is_extension = is_in_extension;
                    }
                    MavXmlElement::Enum => {
                        mavenum = Default::default();
                    }
                    MavXmlElement::Entry => {
                        entry = Default::default();
                    }
                    MavXmlElement::Include => {
                        include = Default::default();
                    }
                    MavXmlElement::Param => {
                        paramid = None;
                    }
                    _ => (),
                }

                stack.push(id);

                for attr in attrs {
                    match stack.last() {
                        Some(&MavXmlElement::Enum) => {
                            if attr.name.local_name == "name" {
                                mavenum.raw_name = attr.value.clone();
                                mavenum.name = rusty_name(&attr.value);
                            }
                            if attr.name.local_name == "bitmask" && attr.value == "true" {
                                mavenum.bitfield = Some("u32".into());
                            }
                        }
                        Some(&MavXmlElement::Entry) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    entry.raw_name = attr.value.clone();
                                    let name = rusty_name(&attr.value);
                                    entry.name = if let Some(n) = name.strip_prefix(&mavenum.name) {
                                        if let Some(ch) = n.chars().next() {
                                            if ch.is_alphabetic() {
                                                n.to_string()
                                            } else {
                                                name
                                            }
                                        } else {
                                            name
                                        }
                                    } else {
                                        name
                                    }
                                }
                                "value" => {
                                    // Deal with hexadecimal numbers
                                    if attr.value.starts_with("0x") {
                                        entry.value = Some(
                                            u32::from_str_radix(
                                                attr.value.trim_start_matches("0x"),
                                                16,
                                            )
                                            .unwrap(),
                                        );
                                    } else {
                                        entry.value = Some(attr.value.parse::<u32>().unwrap());
                                    }
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Message) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    message.raw_name = attr.value.clone();
                                    message.name = rusty_name(&attr.value);
                                }
                                "id" => {
                                    //message.id = attr.value.parse::<u8>().unwrap();
                                    message.id = attr.value.parse::<u32>().unwrap();
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Field) => {
                            match attr.name.local_name.clone().as_ref() {
                                "name" => {
                                    field.raw_name = attr.value.clone();
                                    field.name = snake_name(&attr.value);
                                }
                                "type" => {
                                    field.mavtype = MavType::parse_type(&attr.value).unwrap();
                                }
                                "enum" => {
                                    field.raw_enumtype = Some(attr.value.clone());
                                    field.enumtype = Some(rusty_name(&attr.value));
                                }
                                "display" => {
                                    field.display = Some(attr.value);
                                }
                                _ => (),
                            }
                        }
                        Some(&MavXmlElement::Param) => {
                            if entry.params.is_none() {
                                entry.params = Some(vec![]);
                            }
                            if attr.name.local_name.clone() == "index" {
                                paramid = Some(attr.value.parse::<usize>().unwrap());
                            }
                        }
                        _ => (),
                    }
                }
            }
            Ok(XmlEvent::Characters(s)) => {
                use self::MavXmlElement::*;
                match (stack.last(), stack.get(stack.len() - 2)) {
                    (Some(&Description), Some(&Message)) => {
                        message.description = Some(s.replace("\t", "    "));
                    }
                    (Some(&Field), Some(&Message)) => {
                        field.description = Some(s.replace("\t", "    "));
                    }
                    (Some(&Description), Some(&Enum)) => {
                        mavenum.description = Some(s.replace("\t", "    "));
                    }
                    (Some(&Description), Some(&Entry)) => {
                        entry.description = Some(s.replace("\t", "    "));
                    }
                    (Some(&Param), Some(&Entry)) => {
                        if let Some(ref mut params) = entry.params {
                            // Some messages can jump between values, like:
                            // 0, 1, 2, 7
                            if params.len() < paramid.unwrap() {
                                for index in params.len()..paramid.unwrap() {
                                    params.insert(index, String::from("The use of this parameter (if any), must be defined in the requested message. By default assumed not used (0)."));
                                }
                            }
                            params[paramid.unwrap() - 1] = s;
                        }
                    }
                    (Some(&Include), Some(&Mavlink)) => {
                        include = s.replace("\n", "");
                    }
                    (Some(&Version), Some(&Mavlink)) => {
                        eprintln!("TODO: version {:?}", s);
                    }
                    (Some(&Dialect), Some(&Mavlink)) => {
                        eprintln!("TODO: dialect {:?}", s);
                    }
                    (Some(Deprecated), _) => {
                        eprintln!("TODO: deprecated {:?}", s);
                    }
                    data => {
                        panic!("unexpected text data {:?} reading {:?}", data, s);
                    }
                }
            }
            Ok(XmlEvent::EndElement { .. }) => {
                match stack.last() {
                    Some(&MavXmlElement::Field) => message.fields.push(field.clone()),
                    Some(&MavXmlElement::Entry) => {
                        mavenum.entries.push(entry.clone());
                    }
                    Some(&MavXmlElement::Message) => {
                        is_in_extension = false;
                        // Follow mavlink ordering specification: https://mavlink.io/en/guide/serialization.html#field_reordering
                        let mut not_extension_fields = message.fields.clone();
                        let mut extension_fields = message.fields.clone();

                        not_extension_fields.retain(|field| !field.is_extension);
                        extension_fields.retain(|field| field.is_extension);

                        // Only not mavlink 1 fields need to be sorted
                        not_extension_fields.sort_by(|a, b| a.mavtype.compare(&b.mavtype));

                        // Update msg fields and add the new message
                        let mut msg = message.clone();
                        msg.fields.clear();
                        msg.fields.extend(not_extension_fields);
                        msg.fields.extend(extension_fields);

                        profile.messages.push(msg);
                    }
                    Some(&MavXmlElement::Enum) => {
                        profile.enums.push(mavenum.clone());
                    }
                    Some(&MavXmlElement::Include) => {
                        profile.includes.push(include.clone());
                    }
                    _ => (),
                }
                stack.pop();
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    //let profile = profile.update_messages(); //TODO verify no longer needed
    profile.update_enums()
}

fn merge_enums(profile: &mut MavProfile, modules: &HashMap<String, MavProfile>) {
    fn enum_contains(enums: &[MavEnumEntry], val: u32) -> bool {
        for e in enums {
            if let Some(ev) = e.value {
                if ev == val {
                    return true;
                }
            }
        }
        false
    }
    let mut missing: Vec<MavEnumEntry> = Vec::new();
    for enum_val in &mut profile.enums {
        for inc in &profile.includes {
            for e2 in &modules
                .get(inc)
                .unwrap_or_else(|| panic!("Module {} not loaded!", inc))
                .enums
            {
                if enum_val.name == e2.name {
                    missing.append(
                        &mut e2
                            .entries
                            .iter()
                            .filter(|e| !enum_contains(&enum_val.entries, e.value.unwrap_or(0)))
                            .cloned()
                            .collect(),
                    )
                }
            }
        }
        enum_val.entries.append(&mut missing);
    }
}

/// Generate protobuf represenation of mavlink message set
/// Generate rust representation of mavlink message set with appropriate conversion methods
pub fn generate(
    definitions_dir: &Path,
    definition_file: &OsStr,
    out_dir: &str,
    modules: &mut HashMap<String, MavProfile>,
) {
    let module_name = to_module_name(&definition_file);
    if modules.contains_key(&module_name) {
        return;
    }
    let mut definition_rs = PathBuf::from(&module_name);
    definition_rs.set_extension("rs");
    let mut definition_proto = PathBuf::from(&module_name);
    definition_proto.set_extension("proto");

    let in_path = Path::new(&definitions_dir).join(&definition_file);
    let mut inf = File::open(&in_path).unwrap();

    let dest_path = Path::new(&out_dir)
        .join("src")
        .join("mavlink")
        .join(definition_rs);
    let outf = File::create(&dest_path).unwrap();

    let mut proto_outf = {
        let dest_path = Path::new(&out_dir).join(definition_proto);
        File::create(&dest_path).unwrap()
    };

    let mut profile = parse_profile(&mut inf);
    modules.insert(
        definition_file.to_string_lossy().to_string(),
        profile.clone(),
    );
    for inc in &profile.includes {
        let inc: OsString = inc.into();
        generate(definitions_dir, &inc, out_dir, modules);
    }
    merge_enums(&mut profile, modules);

    // proto file
    write!(proto_outf, "syntax = \"proto3\";\n\n").unwrap();
    write!(proto_outf, "package {};\n\n", module_name).unwrap();
    profile
        .emit_proto(&mut proto_outf, &profile, modules)
        .unwrap();

    // rust file
    let rust_tokens = profile.emit_rust(&module_name);
    writeln!(&outf, "{}", rust_tokens).unwrap();
    match Command::new("rustfmt")
        .arg(dest_path.as_os_str())
        .current_dir(&out_dir)
        .status()
    {
        Ok(_) => (),
        Err(error) => eprintln!("{}", error),
    }

    // Re-run build if definition file changes
    println!("cargo:rerun-if-changed={}", in_path.to_string_lossy());
}

#[cfg(not(feature = "emit-extensions"))]
struct ExtensionFilter {
    pub is_in: bool,
}

struct MavXmlFilter {
    #[cfg(not(feature = "emit-extensions"))]
    extension_filter: ExtensionFilter,
}

impl Default for MavXmlFilter {
    fn default() -> MavXmlFilter {
        MavXmlFilter {
            #[cfg(not(feature = "emit-extensions"))]
            extension_filter: ExtensionFilter { is_in: false },
        }
    }
}

impl MavXmlFilter {
    pub fn filter(&mut self, elements: &mut Vec<Result<XmlEvent, xml::reader::Error>>) {
        // List of filters
        elements.retain(|x| self.filter_extension(x));
    }

    #[cfg(feature = "emit-extensions")]
    pub fn filter_extension(
        &mut self,
        _element: &Result<xml::reader::XmlEvent, xml::reader::Error>,
    ) -> bool {
        return true;
    }

    /// Ignore extension fields
    #[cfg(not(feature = "emit-extensions"))]
    pub fn filter_extension(
        &mut self,
        element: &Result<xml::reader::XmlEvent, xml::reader::Error>,
    ) -> bool {
        match element {
            Ok(content) => {
                match content {
                    XmlEvent::StartElement { name, .. } => {
                        let id = match identify_element(&name.to_string()) {
                            None => {
                                panic!("unexpected element {:?}", name);
                            }
                            Some(kind) => kind,
                        };
                        if id == MavXmlElement::Extensions {
                            self.extension_filter.is_in = true;
                        }
                    }
                    XmlEvent::EndElement { name } => {
                        let id = match identify_element(&name.to_string()) {
                            None => {
                                panic!("unexpected element {:?}", name);
                            }
                            Some(kind) => kind,
                        };

                        if id == MavXmlElement::Message {
                            self.extension_filter.is_in = false;
                        }
                    }
                    _ => {}
                }
                !self.extension_filter.is_in
            }
            Err(error) => panic!("Failed to filter XML: {}", error),
        }
    }
}
