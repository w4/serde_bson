use crate::{byte::BytesLikeBuf, Error};
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Serialize,
};
use std::convert::TryFrom;

pub struct Serializer<'a, B: BytesLikeBuf> {
    pub key: Option<DocumentKey>,
    pub output: &'a mut B,
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

impl<'a, B: BytesLikeBuf> serde::Serializer for Serializer<'a, B> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, B>;
    type SerializeTuple = TupleSerializer<'a, B>;
    type SerializeTupleStruct = TupleStructSerializer<'a, B>;
    type SerializeTupleVariant = TupleVariantSerializer<'a, B>;
    type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = StructSerializer<'a, B>;
    type SerializeStructVariant = StructVariantSerializer<'a, B>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x08, self.key, self.output);
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

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
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
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        // this method ends up very similar to serialize_tuple_variant except string keys
        // are used for the output document

        if self.key.is_some() {
            write_key_or_error!(0x03, self.key, self.output);
        }

        let mut doc_output = start_document(&mut self.output);
        write_key_or_error!(0x03, Some(DocumentKey::Str(variant)), &mut doc_output);
        let nested_doc_output = start_document(&mut doc_output);

        Ok(StructVariantSerializer {
            original_output: self.output,
            nested_doc_output,
            doc_output,
        })
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

pub struct TupleSerializer<'a, B: BytesLikeBuf> {
    inner: SeqSerializer<'a, B>,
}

impl<'a, B: BytesLikeBuf> serde::ser::SerializeTuple for TupleSerializer<'a, B> {
    type Ok = ();
    type Error = <Serializer<'a, B> as serde::Serializer>::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.inner.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}

pub struct TupleVariantSerializer<'a, B: BytesLikeBuf> {
    original_output: &'a mut B,
    array_output: <B::Out as BytesLikeBuf>::Out,
    doc_output: B::Out,
    key: usize,
}

impl<'a, B: BytesLikeBuf> serde::ser::SerializeTupleVariant for TupleVariantSerializer<'a, B> {
    type Ok = ();
    type Error = <Serializer<'a, B> as serde::Serializer>::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

pub struct StructVariantSerializer<'a, B: BytesLikeBuf> {
    original_output: &'a mut B,
    nested_doc_output: <B::Out as BytesLikeBuf>::Out,
    doc_output: B::Out,
}

impl<'a, B: BytesLikeBuf> serde::ser::SerializeStructVariant for StructVariantSerializer<'a, B> {
    type Ok = ();
    type Error = <Serializer<'a, B> as serde::Serializer>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        // we're basically inside a nested StructSerializer here, but we can't
        // instantiate one so we'll duplicate the functionality instead. this
        // is very similar to `TupleVariantSerializer` except string keys are
        // used instead
        value.serialize(Serializer {
            key: Some(DocumentKey::Str(key)),
            output: &mut self.nested_doc_output,
        })?;
        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        // first we close the nested output into the doc output, then write the complete doc output
        // to original_output
        terminate_document(&mut self.doc_output, self.nested_doc_output);
        terminate_document(self.original_output, self.doc_output);
        Ok(())
    }
}

pub struct TupleStructSerializer<'a, B: BytesLikeBuf> {
    inner: SeqSerializer<'a, B>,
}

impl<'a, B: BytesLikeBuf> serde::ser::SerializeTupleStruct for TupleStructSerializer<'a, B> {
    type Ok = ();
    type Error = <Serializer<'a, B> as serde::Serializer>::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.inner.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.inner.end()
    }
}

pub struct SeqSerializer<'a, B: BytesLikeBuf> {
    original_output: &'a mut B,
    doc_output: B::Out,
    key: usize,
}

impl<'a, B: BytesLikeBuf> serde::ser::SerializeSeq for SeqSerializer<'a, B> {
    type Ok = ();
    type Error = <Serializer<'a, B> as serde::Serializer>::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

pub struct StructSerializer<'a, B: BytesLikeBuf> {
    original_output: &'a mut B,
    doc_output: B::Out,
}

impl<'a, B: BytesLikeBuf> serde::ser::SerializeStruct for StructSerializer<'a, B> {
    type Ok = ();
    type Error = <Serializer<'a, B> as serde::Serializer>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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
    pub fn write_to_buf<B: BytesLikeBuf>(&self, buf: &mut B) {
        match self {
            Self::Str(s) => buf.put_slice(s.as_bytes()),
            Self::Int(i) => {
                let mut itoa = itoa::Buffer::new();
                buf.put_slice(itoa.format(*i).as_bytes());
            }
        }
    }
}

pub fn start_document<B: BytesLikeBuf>(buffer: &mut B) -> B::Out {
    let len = buffer.len();

    // splits the output for the doc to be written to, this is appended back onto to the
    // output when `StructSerializer::close` is called.
    let mut doc_output = buffer.split_off(len);

    // reserves a i32 we can write the document size to later
    doc_output.put_i32_le(0);

    doc_output
}

pub fn terminate_document<B: BytesLikeBuf>(original_buffer: &mut B, mut document: B::Out) {
    document.put_u8(0x00); // doc terminator

    // writes the total length of the output to the i32 we reserved earlier
    for (i, byte) in (document.len() as i32).to_le_bytes().iter().enumerate() {
        let byte_ref = document.byte_mut(i);
        debug_assert_eq!(*byte_ref, 0, "document didn't reserve bytes for the length");
        *byte_ref = *byte;
    }

    original_buffer.unsplit(document);
}
