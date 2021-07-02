use crc_any::CRCu16;
use quote::{Ident, Tokens};

use crate::parser::*;
use crate::util::to_module_name;

/// CRC operates over names of the message and names of its fields.
/// Hence we have to preserve the original XML names.
fn extra_crc(msg: &MavMessage) -> u8 {
    // calculate a 8-bit checksum of the key fields of a message, so we
    // can detect incompatible XML changes
    let mut crc = CRCu16::crc16mcrf4cc();

    crc.digest(msg.raw_name.as_bytes());
    crc.digest(" ".as_bytes());

    let mut f = msg.fields.clone();
    // only mavlink 1 fields should be part of the extra_crc
    f.retain(|f| !f.is_extension);
    f.sort_by(|a, b| a.mavtype.compare(&b.mavtype));
    for field in &f {
        crc.digest(field.mavtype.primitive_type().as_bytes());
        crc.digest(" ".as_bytes());
        crc.digest(field.raw_name.as_bytes());
        crc.digest(" ".as_bytes());
        if let MavType::Array(_, size) = field.mavtype {
            crc.digest(&[size as u8]);
        }
    }

    let crcval = crc.get_crc();
    ((crcval & 0xFF) ^ (crcval >> 8)) as u8
}

impl MavProfile {
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
    fn emit_msgs(&self, module_name: &str) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|d| d.emit_rust(module_name))
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
    fn emit_struct_names(&self, module_name: &str) -> Vec<Tokens> {
        self.messages
            .iter()
            .map(|msg| msg.emit_struct_name(module_name))
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

    pub fn emit_rust(&self, module_name: &str) -> Tokens {
        //TODO verify that id_width of u8 is OK even in mavlink v1
        let id_width = Ident::from("u32");

        let comment = self.emit_comments();
        let msgs = self.emit_msgs(module_name);
        let includes = self.emit_includes();
        let enum_names = self.emit_enum_names();
        let struct_names = self.emit_struct_names(module_name);
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
            self.emit_mav_message_default_from_id(&enum_names, &msg_ids, &includes, module_name);
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

            //#[cfg(feature = "serde")]
            //use serde::{Serialize, Deserialize};

            #[cfg(not(feature = "std"))]
            use alloc::vec::Vec;

            #[cfg(not(feature = "std"))]
            use alloc::string::ToString;

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

    fn emit_mav_message(&self, enums: &[Tokens], structs: &[Tokens], includes: &[Ident]) -> Tokens {
        let includes = includes.iter().map(|include| {
            let include_rusty = Ident::from(rusty_name(&include.to_string()));
            quote! {
                #include_rusty(crate::mavlink::#include::MavMessage)
            }
        });

        quote! {
            //#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            //#[cfg_attr(feature = "serde", serde(tag = "type"))]
            pub enum MavMessage {
                #(#enums(#structs),)*
                #(#includes,)*
            }
        }
    }

    fn emit_mav_message_from_includes(&self, includes: &[Ident]) -> Tokens {
        let froms = includes.iter().map(|include| {
            let include_rusty = Ident::from(rusty_name(&include.to_string()));
            quote! {
                impl From<crate::mavlink::#include::MavMessage> for MavMessage {
                    fn from(message: crate::mavlink::#include::MavMessage) -> Self {
                        MavMessage::#include_rusty(message)
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
        let includes_branches = includes.iter().map(|include| {
            let include_rusty = Ident::from(rusty_name(&include.to_string()));
            quote! {
                if let Ok(msg) = crate::mavlink::#include::MavMessage::parse(version, id, payload) {
                    return Ok(MavMessage::#include_rusty(msg))
                }
            }
        });

        quote! {
            fn parse(version: MavlinkVersion, id: #id_width, payload: &[u8]) -> Result<MavMessage, ParserError> {
                match id {
                    #(#ids => #structs::mavlink_deser(version, payload).map(MavMessage::#enums),)*
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

        let includes = includes
            .iter()
            .map(|include| Ident::from(rusty_name(&include.to_string())));

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
        let includes = includes
            .iter()
            .map(|include| Ident::from(rusty_name(&include.to_string())));

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
        module_name: &str,
    ) -> Tokens {
        let module_ident = Ident::from(module_name);
        let data_name = enums
            .iter()
            .map(|enum_name| {
                let name = Ident::from(enum_name.as_str());
                quote!(crate::proto::#module_ident::#name)
            })
            .collect::<Vec<Tokens>>();

        let includes_branches = includes.iter().map(|include| {
            let include_rusty = Ident::from(rusty_name(&include.to_string()));
            quote! {
                if let Ok(msg) = crate::mavlink::#include::MavMessage::default_message_from_id(id) {
                    return Ok(MavMessage::#include_rusty(msg));
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
        let includes = includes
            .iter()
            .map(|include| Ident::from(rusty_name(&include.to_string())));

        quote! {
            fn mavlink_ser(&self) -> Vec<u8> {
                match *self {
                    #(MavMessage::#enums(ref body) => body.mavlink_ser(),)*
                    #(MavMessage::#includes(ref msg) => msg.mavlink_ser(),)*
                }
            }
        }
    }
}

impl MavMessage {
    /// Return Token of "MESSAGE_NAME_DATA
    /// for mavlink struct data
    fn emit_struct_name(&self, module_name: &str) -> Tokens {
        let mut name = String::new();
        name.push_str("crate::proto::");
        name.push_str(module_name);
        name.push_str("::");
        name.push_str(&self.name);
        let name = Ident::from(name);
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

        let encoded_len_name = Ident::from("Self::ENCODED_LEN");

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

    fn emit_rust(&self, module_name: &str) -> Tokens {
        let msg_name = self.emit_struct_name(module_name);
        let (_name_types, msg_encoded_len) = self.emit_name_types();

        let deser_vars = self.emit_deserialize_vars();
        let serialize_vars = self.emit_serialize_vars();

        #[cfg(feature = "emit-description")]
        let description = self.emit_description();

        #[cfg(not(feature = "emit-description"))]

        quote! {
            //XXX proto <-> mav
            impl #msg_name {
                pub const ENCODED_LEN: usize = #msg_encoded_len;

                pub fn mavlink_deser(_version: MavlinkVersion, _input: &[u8]) -> Result<Self, ParserError> {
                    #deser_vars
                }

                pub fn mavlink_ser(&self) -> Vec<u8> {
                    #serialize_vars
                }
            }
        }
    }
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
        let name = "self.".to_string() + &self.name.clone();
        let name = Ident::from(name);
        let buf = Ident::from("_tmp");
        self.mavtype.rust_writer(name, buf)
    }

    /// Emit reader
    fn rust_reader(&self) -> Tokens {
        let name = Ident::from("_struct.".to_string() + &self.name.clone());
        let buf = Ident::from("buf");
        if let Some(enum_name) = &self.enumtype {
            if let MavType::Array(_t, _size) = &self.mavtype {
                return self.mavtype.rust_reader(name, buf, false);
            }
            // handle enum by FromPrimitive
            let tmp = self.mavtype.rust_reader(Ident::from("let tmp"), buf, false);
            let val = Ident::from("from_".to_string() + &self.mavtype.rust_type());
            quote!(
                #tmp
                #name = FromPrimitive::#val(tmp)
                    .ok_or(ParserError::InvalidEnum { enum_type: #enum_name.to_string(), value: tmp as u32 })?;
            )
        } else {
            self.mavtype.rust_reader(name, buf, true)
        }
    }
}

impl MavType {
    /// Emit reader of a given type
    pub fn rust_reader(&self, val: Ident, buf: Ident, with_cast: bool) -> Tokens {
        use self::MavType::*;
        match self.clone() {
            Char if with_cast => quote! {#val = #buf.get_u8() as u32;},
            Char => quote! {#val = #buf.get_u8();},
            UInt8 if with_cast => quote! {#val = #buf.get_u8() as u32;},
            UInt8 => quote! {#val = #buf.get_u8();},
            UInt16 if with_cast => quote! {#val = #buf.get_u16_le() as u32;},
            UInt16 => quote! {#val = #buf.get_u16_le();},
            UInt32 => quote! {#val = #buf.get_u32_le();},
            UInt64 => quote! {#val = #buf.get_u64_le();},
            UInt8MavlinkVersion if with_cast => quote! {#val = #buf.get_u8() as u32;},
            UInt8MavlinkVersion => quote! {#val = #buf.get_u8();},
            Int8 if with_cast => quote! {#val = #buf.get_i8() as i32;},
            Int8 => quote! {#val = #buf.get_i8();},
            Int16 if with_cast => quote! {#val = #buf.get_i16_le() as i32;},
            Int16 => quote! {#val = #buf.get_i16_le();},
            Int32 => quote! {#val = #buf.get_i32_le();},
            Int64 => quote! {#val = #buf.get_i64_le();},
            Float => quote! {#val = #buf.get_f32_le();},
            Double => quote! {#val = #buf.get_f64_le();},
            Array(t, size) => {
                if let Char = *t {
                    quote! {
                        let mut s = Vec::with_capacity(#size);
                        for _ in 0..#size {
                            s.push(#buf.get_u8());
                        }
                        #val = String::from_utf8_lossy(&s).into();
                    }
                } else {
                    // it is a vector
                    let r = t.rust_reader(Ident::from("let val"), buf, with_cast);
                    quote! {
                            for _ in 0..#size {
                                #r
                    #[allow(clippy::useless_conversion)]
                                #val.push(val.into());
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
            UInt8MavlinkVersion => quote! {#buf.put_u8(#val as u8);},
            UInt8 => quote! {#buf.put_u8(#val as u8);},
            Char => quote! {#buf.put_u8(#val as u8);}, //XXX already u8
            UInt16 => quote! {#buf.put_u16_le(#val as u16);},
            UInt32 => quote! {#buf.put_u32_le(#val as u32);},
            Int8 => quote! {#buf.put_i8(#val as i8);},
            Int16 => quote! {#buf.put_i16_le(#val as i16);},
            Int32 => quote! {#buf.put_i32_le(#val as i32);},
            Float => quote! {#buf.put_f32_le(#val as f32);},
            UInt64 => quote! {#buf.put_u64_le(#val as u64);},
            Int64 => quote! {#buf.put_i64_le(#val as i64);},
            Double => quote! {#buf.put_f64_le(#val as f64);},
            Array(t, _size) => {
                if let Char = *t {
                    quote! {
                        for val in #val.bytes() {
                            #buf.put_u8(val);
                        }
                    }
                } else {
                    let w = t.rust_writer(Ident::from("*val"), buf);
                    quote! {
                        for val in &#val {
                            #w
                        }
                    }
                }
            }
        }
    }
}
