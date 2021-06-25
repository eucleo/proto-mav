use crc_any::CRCu16;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::default::Default;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::u32;

use xml::reader::{EventReader, XmlEvent};

use quote::{Ident, Tokens};

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
                                    enm.bitfield = Some(field.mavtype.rust_type());
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

    /// Simple header comment
    fn emit_comments(&self) -> Ident {
        Ident::from("// This file was automatically generated, do not edit \n".to_string())
    }

    /// Emit includes
    fn emit_includes(&self) -> Vec<Ident> {
        self.includes
            .iter()
            .map(|i| Ident::from(to_module_name(i)))
            .collect::<Vec<Ident>>()
    }

    /// Emit rust messages
    fn emit_msgs(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|d| d.emit_rust())
            .collect::<Vec<Tokens>>()
    }

    /// Emit rust enums
    fn emit_enums(&self) -> Vec<Tokens> {
        self.enums
            .iter()
            .map(|d| d.emit_rust())
            .collect::<Vec<Tokens>>()
    }

    /// Get list of original message names
    fn emit_enum_names(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| {
                let name = Ident::from(msg.name.clone());
                quote!(#name)
            })
            .collect::<Vec<Tokens>>()
    }

    /// Emit message names with "_DATA" at the end
    fn emit_struct_names(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| msg.emit_struct_name())
            .collect::<Vec<Tokens>>()
    }

    /// A list of message IDs
    fn emit_msg_ids(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| {
                let id = Ident::from(msg.id.to_string());
                quote!(#id)
            })
            .collect::<Vec<Tokens>>()
    }

    /// CRC values needed for mavlink parsing
    fn emit_msg_crc(&self) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| {
                let crc = Ident::from(extra_crc(&msg).to_string());
                quote!(#crc)
            })
            .collect::<Vec<Tokens>>()
    }

    fn emit_rust(&self) -> Tokens {
        //TODO verify that id_width of u8 is OK even in mavlink v1
        let id_width = Ident::from("u32");

        let comment = self.emit_comments();
        let msgs = self.emit_msgs();
        let includes = self.emit_includes();
        let enum_names = self.emit_enum_names();
        let struct_names = self.emit_struct_names();
        let enums = self.emit_enums();
        let msg_ids = self.emit_msg_ids();
        let msg_crc = self.emit_msg_crc();

        let mav_message = self.emit_mav_message(&enum_names, &struct_names, &includes);
        let mav_message_from_includes = self.emit_mav_message_from_includes(&includes);
        let mav_message_parse =
            self.emit_mav_message_parse(&enum_names, &struct_names, &msg_ids, &includes);
        let mav_message_crc = self.emit_mav_message_crc(&id_width, &msg_ids, &msg_crc, &includes);
        let mav_message_name = self.emit_mav_message_name(&enum_names, &includes);
        let mav_message_id = self.emit_mav_message_id(&enum_names, &msg_ids, &includes);
        let mav_message_id_from_name =
            self.emit_mav_message_id_from_name(&enum_names, &msg_ids, &includes);
        let mav_message_default_from_id =
            self.emit_mav_message_default_from_id(&enum_names, &msg_ids, &includes);
        let mav_message_serialize = self.emit_mav_message_serialize(&enum_names, &includes);

        quote! {
            #comment
            use crate::MavlinkVersion;
            #[allow(unused_imports)]
            use bytes::{Buf, BufMut, Bytes, BytesMut};
            #[allow(unused_imports)]
            use num_derive::FromPrimitive;
            #[allow(unused_imports)]
            use num_traits::FromPrimitive;
            #[allow(unused_imports)]
            use num_derive::ToPrimitive;
            #[allow(unused_imports)]
            use num_traits::ToPrimitive;
            #[allow(unused_imports)]
            use bitflags::bitflags;

            use crate::{Message, error::*};
            #[allow(unused_imports)]
            use crate::{#(mavlink::#includes::*),*};

            #[cfg(feature = "serde")]
            use serde::{Serialize, Deserialize};

            #[cfg(not(feature = "std"))]
            use alloc::vec::Vec;

            #[cfg(not(feature = "std"))]
            use alloc::string::ToString;

            #(#enums)*

            #(#msgs)*

            #[derive(Clone, PartialEq, Debug)]
            #mav_message

            #mav_message_from_includes

            impl Message for MavMessage {
                #mav_message_parse
                #mav_message_name
                #mav_message_id
                #mav_message_id_from_name
                #mav_message_default_from_id
                #mav_message_serialize
                #mav_message_crc
            }
        }
    }

    fn emit_proto(
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

    fn emit_mav_message(&self, enums: &[Tokens], structs: &[Tokens], includes: &[Ident]) -> Tokens {
        let includes = includes.iter().map(|include| {
            quote! {
                #include(crate::mavlink::#include::MavMessage)
            }
        });

        quote! {
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            #[cfg_attr(feature = "serde", serde(tag = "type"))]
            pub enum MavMessage {
                #(#enums(#structs),)*
                #(#includes,)*
            }
        }
    }

    fn emit_mav_message_from_includes(&self, includes: &[Ident]) -> Tokens {
        let froms = includes.iter().map(|include| {
            quote! {
                impl From<crate::mavlink::#include::MavMessage> for MavMessage {
                    fn from(message: crate::mavlink::#include::MavMessage) -> Self {
                        MavMessage::#include(message)
                    }
                }
            }
        });

        quote! {
            #(#froms)*
        }
    }

    fn emit_mav_message_parse(
        &self,
        enums: &[Tokens],
        structs: &[Tokens],
        ids: &[Tokens],
        includes: &[Ident],
    ) -> Tokens {
        let id_width = Ident::from("u32");

        // try parsing all included message variants if it doesn't land in the id
        // range for this message
        let includes_branches = includes.iter().map(|i| {
            quote! {
                if let Ok(msg) = crate::mavlink::#i::MavMessage::parse(version, id, payload) {
                    return Ok(MavMessage::#i(msg))
                }
            }
        });

        quote! {
            fn parse(version: MavlinkVersion, id: #id_width, payload: &[u8]) -> Result<MavMessage, ParserError> {
                match id {
                    #(#ids => #structs::deser(version, payload).map(MavMessage::#enums),)*
                    _ => {
                        #(#includes_branches)*
                        Err(ParserError::UnknownMessage { id })
                    },
                }
            }
        }
    }

    fn emit_mav_message_crc(
        &self,
        id_width: &Ident,
        ids: &[Tokens],
        crc: &[Tokens],
        includes: &[Ident],
    ) -> Tokens {
        let includes_branch = includes.iter().map(|include| {
            quote! {
                match crate::mavlink::#include::MavMessage::extra_crc(id) {
                    0 => {},
                    any => return any
                }
            }
        });

        quote! {
            fn extra_crc(id: #id_width) -> u8 {
                match id {
                    #(#ids => #crc,)*
                    _ => {
                        #(#includes_branch)*

                        0
                    },
                }
            }
        }
    }

    fn emit_mav_message_name(&self, enums: &[Tokens], includes: &[Ident]) -> Tokens {
        let enum_names = enums
            .iter()
            .map(|enum_name| {
                let name = Ident::from(format!("\"{}\"", enum_name));
                quote!(#name)
            })
            .collect::<Vec<Tokens>>();

        quote! {
            fn message_name(&self) -> &'static str {
                match self {
                    #(MavMessage::#enums(..) => #enum_names,)*
                    #(MavMessage::#includes(msg) => msg.message_name(),)*
                }
            }
        }
    }

    fn emit_mav_message_id(&self, enums: &[Tokens], ids: &[Tokens], includes: &[Ident]) -> Tokens {
        let id_width = Ident::from("u32");
        quote! {
            fn message_id(&self) -> #id_width {
                match self {
                    #(MavMessage::#enums(..) => #ids,)*
                    #(MavMessage::#includes(msg) => msg.message_id(),)*
                }
            }
        }
    }

    fn emit_mav_message_id_from_name(
        &self,
        enums: &[Tokens],
        ids: &[Tokens],
        includes: &[Ident],
    ) -> Tokens {
        let includes_branch = includes.iter().map(|include| {
            quote! {
                match crate::mavlink::#include::MavMessage::message_id_from_name(name) {
                    Ok(name) => return Ok(name),
                    Err(..) => {}
                }
            }
        });

        let enum_names = enums
            .iter()
            .map(|enum_name| {
                let name = Ident::from(format!("\"{}\"", enum_name));
                quote!(#name)
            })
            .collect::<Vec<Tokens>>();

        quote! {
            fn message_id_from_name(name: &str) -> Result<u32, &'static str> {
                match name {
                    #(#enum_names => Ok(#ids),)*
                    _ => {
                        #(#includes_branch)*

                        Err("Invalid message name.")
                    }
                }
            }
        }
    }

    fn emit_mav_message_default_from_id(
        &self,
        enums: &[Tokens],
        ids: &[Tokens],
        includes: &[Ident],
    ) -> Tokens {
        let data_name = enums
            .iter()
            .map(|enum_name| {
                //let name = Ident::from(format!("{}_DATA", enum_name));
                let name = Ident::from(enum_name.as_str());
                quote!(#name)
            })
            .collect::<Vec<Tokens>>();

        let includes_branches = includes.iter().map(|include| {
            quote! {
                if let Ok(msg) = crate::mavlink::#include::MavMessage::default_message_from_id(id) {
                    return Ok(MavMessage::#include(msg));
                }
            }
        });

        quote! {
            fn default_message_from_id(id: u32) -> Result<MavMessage, &'static str> {
                match id {
                    #(#ids => Ok(MavMessage::#enums(#data_name::default())),)*
                    _ => {
                        #(#includes_branches)*

                        Err("Invalid message id.")
                    }
                }
            }
        }
    }

    fn emit_mav_message_serialize(&self, enums: &[Tokens], includes: &[Ident]) -> Tokens {
        quote! {
            fn ser(&self) -> Vec<u8> {
                match *self {
                    #(MavMessage::#enums(ref body) => body.ser(),)*
                    #(MavMessage::#includes(ref msg) => msg.ser(),)*
                }
            }
        }
    }
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

impl MavEnum {
    fn has_enum_values(&self) -> bool {
        self.entries.iter().all(|x| x.value.is_some())
    }

    fn emit_defs(&self) -> Vec<Tokens> {
        let mut cnt = 0;
        self.entries
            .iter()
            .map(|enum_entry| {
                let name = if self.bitfield.is_some() {
                    Ident::from(enum_entry.name.to_uppercase())
                } else {
                    Ident::from(enum_entry.name.clone())
                };
                let value;
                if !self.has_enum_values() {
                    value = Ident::from(cnt.to_string());
                    cnt += 1;
                } else {
                    value = Ident::from(enum_entry.value.unwrap().to_string());
                };
                if self.bitfield.is_some() {
                    quote!(const #name = #value;)
                } else {
                    quote!(#name = #value,)
                }
            })
            .collect::<Vec<Tokens>>()
    }

    fn emit_name(&self) -> Tokens {
        let name = Ident::from(self.name.clone());
        quote!(#name)
    }

    fn emit_rust(&self) -> Tokens {
        let defs = self.emit_defs();
        //let default = Ident::from(self.entries[0].name.clone());
        let default = if self.bitfield.is_some() {
            Ident::from(self.entries[0].name.to_uppercase())
        } else {
            Ident::from(self.entries[0].name.clone())
        };
        let enum_name = self.emit_name();

        let enum_def;
        if let Some(width) = self.bitfield.clone() {
            let width = Ident::from(width);
            enum_def = quote! {
                bitflags!{
                    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
                    pub struct #enum_name: #width {
                        #(#defs)*
                    }
                }
            };
        } else {
            enum_def = quote! {
                #[derive(Debug, Copy, Clone, PartialEq, FromPrimitive, ToPrimitive)]
                #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
                #[cfg_attr(feature = "serde", serde(tag = "type"))]
                pub enum #enum_name {
                    #(#defs)*
                }
            };
        }

        quote! {
            #enum_def

            impl Default for #enum_name {
                fn default() -> Self {
                    #enum_name::#default
                }
            }
        }
    }

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

impl MavMessage {
    /// Return Token of "MESSAGE_NAME_DATA
    /// for mavlink struct data
    fn emit_struct_name(&self) -> Tokens {
        let name = Ident::from(self.name.clone());
        quote!(#name)
    }

    fn emit_name_types(&self) -> (Vec<Tokens>, usize) {
        let mut encoded_payload_len: usize = 0;
        let field_toks = self
            .fields
            .iter()
            .map(|field| {
                let nametype = field.emit_name_type();
                encoded_payload_len += field.mavtype.len();

                #[cfg(feature = "emit-description")]
                let description = field.emit_description();

                #[cfg(not(feature = "emit-description"))]
                let description = Ident::from("");

                quote! {
                    #description
                    #nametype
                }
            })
            .collect::<Vec<Tokens>>();
        (field_toks, encoded_payload_len)
    }

    /// Generate description for the given message
    #[cfg(feature = "emit-description")]
    fn emit_description(&self) -> Tokens {
        let mut desc = String::from(format!("\n/// id: {}\n", self.id));
        if let Some(val) = self.description.clone() {
            desc = desc + &format!("/// {}.\n", val);
        }
        let desc = Ident::from(desc);
        quote!(#desc)
    }

    fn emit_serialize_vars(&self) -> Tokens {
        let ser_vars = self
            .fields
            .iter()
            .map(|f| f.rust_writer())
            .collect::<Vec<Tokens>>();
        quote! {
            let mut _tmp = Vec::new();
            #(#ser_vars)*
            _tmp
        }
    }

    fn emit_deserialize_vars(&self) -> Tokens {
        let deser_vars = self
            .fields
            .iter()
            .map(|f| f.rust_reader())
            .collect::<Vec<Tokens>>();

        //let encoded_len_name = Ident::from(format!("{}_DATA::ENCODED_LEN", self.name));
        let encoded_len_name = Ident::from(format!("{}::ENCODED_LEN", self.name));

        if deser_vars.is_empty() {
            // struct has no fields
            quote! {
                Ok(Self::default())
            }
        } else {
            // Should look at getting rid of the #[allow... below but it is non-trivial.
            quote! {
                let avail_len = _input.len();

                // fast zero copy
                let mut buf = BytesMut::from(_input);

                // handle payload length truncuation due to empty fields
                if avail_len < #encoded_len_name {
                    //copy available bytes into an oversized buffer filled with zeros
                    let mut payload_buf  = [0; #encoded_len_name];
                    payload_buf[0..avail_len].copy_from_slice(_input);
                    buf = BytesMut::from(&payload_buf[..]);
                }

                #[allow(clippy::field_reassign_with_default)]
                {
                    let mut _struct = Self::default();
                    #(#deser_vars)*
                    Ok(_struct)
                }
            }
        }
    }

    fn emit_rust(&self) -> Tokens {
        let msg_name = self.emit_struct_name();
        let (name_types, msg_encoded_len) = self.emit_name_types();

        let deser_vars = self.emit_deserialize_vars();
        let serialize_vars = self.emit_serialize_vars();

        #[cfg(feature = "emit-description")]
        let description = self.emit_description();

        #[cfg(not(feature = "emit-description"))]
        let description = Ident::from("");

        quote! {
            #description
            #[derive(Debug, Clone, PartialEq, Default)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct #msg_name {
                #(#name_types)*
            }

            impl #msg_name {
                pub const ENCODED_LEN: usize = #msg_encoded_len;

                pub fn deser(version: MavlinkVersion, _input: &[u8]) -> Result<Self, ParserError> {
                    #deser_vars
                }

                pub fn ser(&self) -> Vec<u8> {
                    #serialize_vars
                }
            }
        }
    }

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

impl MavField {
    /// Emit rust name of a given field
    fn emit_name(&self) -> Tokens {
        let name = Ident::from(self.name.clone());
        quote!(#name)
    }

    /// Emit rust type of the field
    fn emit_type(&self) -> Tokens {
        let mavtype;
        match self.mavtype {
            MavType::Array(_, _) => {
                mavtype = Ident::from(self.mavtype.rust_type());
            }
            _ => match self.enumtype {
                Some(ref enumname) => {
                    mavtype = Ident::from(enumname.clone());
                }
                _ => {
                    mavtype = Ident::from(self.mavtype.rust_type());
                }
            },
        }
        quote!(#mavtype)
    }

    /// Generate description for the given field
    #[cfg(feature = "emit-description")]
    fn emit_description(&self) -> Tokens {
        let mut desc = Vec::new();
        if let Some(val) = self.description.clone() {
            desc.push(format!("\n/// {}.", val));
        }
        desc.push("\n".to_string());
        let desc: String = desc.iter().map(|s| s.to_string()).collect();
        let desc = Ident::from(desc);
        quote!(#desc)
    }

    /// Combine rust name and type of a given field
    fn emit_name_type(&self) -> Tokens {
        let name = self.emit_name();
        let fieldtype = self.emit_type();
        quote!(pub #name: #fieldtype,)
    }

    /// Emit writer
    fn rust_writer(&self) -> Tokens {
        let mut name = "self.".to_string() + &self.name.clone();
        if self.enumtype.is_some() {
            if let Some(dsp) = &self.display {
                // potentially a bitflag
                if dsp == "bitmask" {
                    // it is a bitflag
                    name += ".bits()";
                } else {
                    panic!("Display option not implemented");
                }
            } else {
                match self.mavtype {
                    MavType::Array(_, _) => {} // cast are not necessary for arrays
                    _ => {
                        // an enum, have to use "*foo as u8" cast
                        name += " as ";
                        name += &self.mavtype.rust_type();
                    }
                }
            }
        }
        let name = Ident::from(name);
        let buf = Ident::from("_tmp");
        self.mavtype.rust_writer(name, buf)
    }

    /// Emit reader
    fn rust_reader(&self) -> Tokens {
        let name = Ident::from("_struct.".to_string() + &self.name.clone());
        let buf = Ident::from("buf");
        if let Some(enum_name) = &self.enumtype {
            if let Some(dsp) = &self.display {
                if dsp == "bitmask" {
                    // bitflags
                    let tmp = self.mavtype.rust_reader(Ident::from("let tmp"), buf);
                    let enum_name_ident = Ident::from(enum_name.clone());
                    quote! {
                        #tmp
                        #name = #enum_name_ident::from_bits(tmp & #enum_name_ident::all().bits())
                            .ok_or(ParserError::InvalidFlag { flag_type: #enum_name.to_string(), value: tmp as u32 })?;
                    }
                } else {
                    panic!("Display option not implemented");
                }
            } else {
                if let MavType::Array(_t, _size) = &self.mavtype {
                    return self.mavtype.rust_reader(name, buf);
                }
                // handle enum by FromPrimitive
                let tmp = self.mavtype.rust_reader(Ident::from("let tmp"), buf);
                let val = Ident::from("from_".to_string() + &self.mavtype.rust_type());
                quote!(
                    #tmp
                    #name = FromPrimitive::#val(tmp)
                        .ok_or(ParserError::InvalidEnum { enum_type: #enum_name.to_string(), value: tmp as u32 })?;
                )
            }
        } else {
            self.mavtype.rust_reader(name, buf)
        }
    }

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
    fn parse_type(s: &str) -> Option<MavType> {
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

    /// Emit reader of a given type
    pub fn rust_reader(&self, val: Ident, buf: Ident) -> Tokens {
        use self::MavType::*;
        match self.clone() {
            Char => quote! {#val = #buf.get_u8() as char;},
            UInt8 => quote! {#val = #buf.get_u8();},
            UInt16 => quote! {#val = #buf.get_u16_le();},
            UInt32 => quote! {#val = #buf.get_u32_le();},
            UInt64 => quote! {#val = #buf.get_u64_le();},
            UInt8MavlinkVersion => quote! {#val = #buf.get_u8();},
            Int8 => quote! {#val = #buf.get_i8();},
            Int16 => quote! {#val = #buf.get_i16_le();},
            Int32 => quote! {#val = #buf.get_i32_le();},
            Int64 => quote! {#val = #buf.get_i64_le();},
            Float => quote! {#val = #buf.get_f32_le();},
            Double => quote! {#val = #buf.get_f64_le();},
            Array(t, size) => {
                if size > 32 {
                    // it is a vector
                    let r = t.rust_reader(Ident::from("let val"), buf);
                    quote! {
                        for _ in 0..#size {
                            #r
                            #val.push(val);
                        }
                    }
                } else {
                    // handle as a slice
                    let r = t.rust_reader(Ident::from("let val"), buf);
                    quote! {
                        for idx in 0..#size {
                            #r
                            #val[idx] = val;
                        }
                    }
                }
            }
        }
    }

    /// Emit writer of a given type
    pub fn rust_writer(&self, val: Ident, buf: Ident) -> Tokens {
        use self::MavType::*;
        match self.clone() {
            UInt8MavlinkVersion => quote! {#buf.put_u8(#val);},
            UInt8 => quote! {#buf.put_u8(#val);},
            Char => quote! {#buf.put_u8(#val as u8);},
            UInt16 => quote! {#buf.put_u16_le(#val);},
            UInt32 => quote! {#buf.put_u32_le(#val);},
            Int8 => quote! {#buf.put_i8(#val);},
            Int16 => quote! {#buf.put_i16_le(#val);},
            Int32 => quote! {#buf.put_i32_le(#val);},
            Float => quote! {#buf.put_f32_le(#val);},
            UInt64 => quote! {#buf.put_u64_le(#val);},
            Int64 => quote! {#buf.put_i64_le(#val);},
            Double => quote! {#buf.put_f64_le(#val);},
            Array(t, _size) => {
                let w = t.rust_writer(Ident::from("*val"), buf);
                quote! {
                    for val in &#val {
                        #w
                    }
                }
            }
        }
    }

    /// Size of a given Mavtype
    fn len(&self) -> usize {
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
    fn order_len(&self) -> usize {
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

    /// Return protobuf equivalent of a given Mavtype
    /// Used for generating proto message fields.
    pub fn proto_type(&self) -> String {
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

fn rusty_name(name: &str) -> String {
    name.split('_')
        .map(|x| x.to_lowercase())
        .map(|x| {
            let mut v: Vec<char> = x.chars().collect();
            v[0] = v[0].to_uppercase().next().unwrap();
            v.into_iter().collect()
        })
        .collect::<Vec<String>>()
        .join("")
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
                                    field.name = attr.value.clone();
                                    if field.name == "type" {
                                        field.name = "mavtype".to_string();
                                    }
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
    let rust_tokens = profile.emit_rust();
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

/// CRC operates over names of the message and names of its fields
/// Hence we have to preserve the original uppercase names delimited with an underscore
/// For field names, we replace "type" with "mavtype" to make it rust compatible (this is
/// needed for generating sensible rust code), but for calculating crc function we have to
/// use the original name "type"
pub fn extra_crc(msg: &MavMessage) -> u8 {
    // calculate a 8-bit checksum of the key fields of a message, so we
    // can detect incompatible XML changes
    let mut crc = CRCu16::crc16mcrf4cc();

    crc.digest(msg.name.as_bytes());
    crc.digest(" ".as_bytes());

    let mut f = msg.fields.clone();
    // only mavlink 1 fields should be part of the extra_crc
    f.retain(|f| !f.is_extension);
    f.sort_by(|a, b| a.mavtype.compare(&b.mavtype));
    for field in &f {
        crc.digest(field.mavtype.primitive_type().as_bytes());
        crc.digest(" ".as_bytes());
        if field.name == "mavtype" {
            crc.digest("type".as_bytes());
        } else {
            crc.digest(field.name.as_bytes());
        }
        crc.digest(" ".as_bytes());
        if let MavType::Array(_, size) = field.mavtype {
            crc.digest(&[size as u8]);
        }
    }

    let crcval = crc.get_crc();
    ((crcval & 0xFF) ^ (crcval >> 8)) as u8
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
