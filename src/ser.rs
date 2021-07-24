use crate::Error;
use bytes::{BufMut, BytesMut};
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Serialize,
};
use std::convert::TryFrom;

pub struct Serializer<'a> {
    pub key: Option<DocumentKey>,
    pub output: &'a mut BytesMut,
}

macro_rules! write_key_or_error {
    ($id:literal, $key:expr, $output:expr) => {
        if let Some(key) = $key {
            $output.put_u8($id);
            key.write_to_buf($output);
            $output.put_u8(0x00);
        } else {
            return Err(Error::NotSerializingStruct);
        }
    };
}

impl<'a> serde::Serializer for Serializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = TupleSerializer<'a>;
    type SerializeTupleStruct = TupleStructSerializer<'a>;
    type SerializeTupleVariant = TupleVariantSerializer<'a>;
    type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x01, self.key, self.output);
        self.output.put_u8(v as u8);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x10, self.key, self.output);
        self.output.put_i32_le(v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x12, self.key, self.output);
        self.output.put_i64_le(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x01, self.key, self.output);
        self.output.put_f64_le(v);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x02, self.key, self.output);

        let v = v.as_bytes();
        let len = i32::try_from(v.len() + 1) // `+ 1` for the null byte at the end of the str
            .unwrap_or_else(|_| panic!(
                "encoded string exceeds max size: {}",
                i32::MAX - 1
            ));

        self.output.put_i32_le(len);
        self.output.put_slice(v);
        self.output.put_u8(0x00);

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x05, self.key, self.output);

        // we don't need the + 1 here since there's no null terminator
        let len = i32::try_from(v.len())
            .unwrap_or_else(|_| panic!("bytes exceeds max size: {}", i32::MAX));

        self.output.put_i32_le(len);
        self.output.put_u8(0x00); // subtype, we'll just assume 0x00
        self.output.put_slice(v);

        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x0A, self.key, self.output);
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let mut struct_serializer = self.serialize_struct("", 0)?;
        struct_serializer.serialize_field(variant, value)?;
        struct_serializer.end()
    }

    fn serialize_seq(mut self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        // it'd be so much simpler if we could just delegate SerializeSeq to SerializeStruct since
        // an array in bson is just a document with numeric keys but SerializeStruct needs a
        // &'static str, and we can't do that unless we either write the string repr of 1..i32::MAX
        // to the binary or leak the string, neither seem like a good idea.

        if self.key.is_some() {
            write_key_or_error!(0x04, self.key, self.output);
        }

        let doc_output = start_document(&mut self.output);

        Ok(SeqSerializer {
            original_output: self.output,
            doc_output,
            key: 0,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer {
            inner: self.serialize_seq(Some(len))?,
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructSerializer {
            inner: self.serialize_seq(Some(len))?,
        })
    }

    fn serialize_tuple_variant(
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        // we essentially need a nested type here which isn't too well supported with serde's
        // model, first we create a new document which we'll use to write the variant into as
        // the key, and then we'll create a second document we'll use as an array for each
        // tuple element

        if self.key.is_some() {
            write_key_or_error!(0x03, self.key, self.output);
        }

        let mut doc_output = start_document(&mut self.output);
        write_key_or_error!(0x04, Some(DocumentKey::Str(variant)), &mut doc_output);
        let array_output = start_document(&mut doc_output);

        Ok(TupleVariantSerializer {
            original_output: self.output,
            array_output,
            doc_output,
            key: 0,
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!("map")
    }

    fn serialize_struct(
        mut self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        if self.key.is_some() {
            write_key_or_error!(0x03, self.key, self.output);
        }

        let doc_output = start_document(&mut self.output);

        Ok(StructSerializer {
            original_output: self.output,
            doc_output,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!("struct variant")
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsignedIntNotInSpec)
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsignedIntNotInSpec)
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsignedIntNotInSpec)
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsignedIntNotInSpec)
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsignedIntNotInSpec)
    }
}

pub struct TupleSerializer<'a> {
    inner: SeqSerializer<'a>,
}

impl<'a> serde::ser::SerializeTuple for TupleSerializer<'a> {
    type Ok = ();
    type Error = <Serializer<'a> as serde::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.inner.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}

pub struct TupleVariantSerializer<'a> {
    original_output: &'a mut BytesMut,
    array_output: BytesMut,
    doc_output: BytesMut,
    key: usize,
}

impl<'a> serde::ser::SerializeTupleVariant for TupleVariantSerializer<'a> {
    type Ok = ();
    type Error = <Serializer<'a> as serde::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // we're basically inside a SeqSerializer here, but we can't instantiate one
        // so we'll duplicate the functionality instead
        value.serialize(Serializer {
            key: Some(DocumentKey::Int(self.key)),
            output: &mut self.array_output,
        })?;
        self.key += 1;
        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        // first we close the array output into the doc output, then write the complete doc output
        // to original_output
        terminate_document(&mut self.doc_output, self.array_output);
        terminate_document(self.original_output, self.doc_output);
        Ok(())
    }
}

pub struct TupleStructSerializer<'a> {
    inner: SeqSerializer<'a>,
}

impl<'a> serde::ser::SerializeTupleStruct for TupleStructSerializer<'a> {
    type Ok = ();
    type Error = <Serializer<'a> as serde::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.inner.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}

pub struct SeqSerializer<'a> {
    original_output: &'a mut BytesMut,
    doc_output: BytesMut,
    key: usize,
}

impl<'a> serde::ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = ();
    type Error = <Serializer<'a> as serde::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(Serializer {
            key: Some(DocumentKey::Int(self.key)),
            output: &mut self.doc_output,
        })?;
        self.key += 1;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        terminate_document(self.original_output, self.doc_output);
        Ok(())
    }
}

pub struct StructSerializer<'a> {
    original_output: &'a mut BytesMut,
    doc_output: BytesMut,
}

impl<'a> serde::ser::SerializeStruct for StructSerializer<'a> {
    type Ok = ();
    type Error = <Serializer<'a> as serde::Serializer>::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(Serializer {
            key: Some(DocumentKey::Str(key)),
            output: &mut self.doc_output,
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        terminate_document(self.original_output, self.doc_output);
        Ok(())
    }
}

pub enum DocumentKey {
    Str(&'static str),
    Int(usize),
}

impl DocumentKey {
    pub fn write_to_buf(&self, buf: &mut BytesMut) {
        match self {
            Self::Str(s) => buf.put_slice(s.as_bytes()),
            Self::Int(i) => {
                let mut itoa = itoa::Buffer::new();
                buf.put_slice(itoa.format(*i).as_bytes());
            }
        }
    }
}

pub fn start_document(buffer: &mut BytesMut) -> BytesMut {
    // splits the output for the doc to be written to, this is appended back onto to the
    // output when `StructSerializer::close` is called.
    let mut doc_output = buffer.split_off(buffer.len());

    // reserves a i32 we can write the document size to later
    doc_output.put_i32(0);

    doc_output
}

pub fn terminate_document(original_buffer: &mut BytesMut, mut document: BytesMut) {
    document.put_u8(0x00); // doc terminator

    // writes the total length of the output to the i32 we reserved earlier
    for (i, byte) in (document.len() as i32).to_le_bytes().iter().enumerate() {
        debug_assert_eq!(
            document[i], 0,
            "document didn't reserve bytes for the length"
        );
        document[i] = *byte;
    }

    original_buffer.unsplit(document);
}
