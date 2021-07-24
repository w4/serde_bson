use crate::Error;
use bytes::{BufMut, BytesMut};
use serde::Serialize;
use std::convert::TryFrom;

pub struct Serializer<'a> {
    pub key: Option<&'static str>,
    pub output: &'a mut BytesMut,
}

macro_rules! write_key_or_error {
    ($id:literal, $key:expr, $output:expr) => {
        if let Some(key) = $key {
            $output.put_u8($id);
            $output.put_slice(key.as_bytes());
            $output.put_u8(0x00);
        } else {
            return Err(Error::NotSerializingStruct);
        }
    };
}

impl<'a> serde::Serializer for Serializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
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

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        write_key_or_error!(0x01, self.key, self.output);
        self.output.put_f64_le(v);
        Ok(())
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(Error::UnsignedIntNotInSpec)
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
        unimplemented!("unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!("unit variant")
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
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!("newtype variant")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        todo!("seq")
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!("tuple")
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!("tuple struct")
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!("tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!("map")
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        if self.key.is_some() {
            write_key_or_error!(0x03, self.key, self.output);
        }

        // splits the output for the doc to be written to, this is appended back onto to the
        // output when `StructSerializer::close` is called.
        let mut doc_output = self.output.split_off(self.output.len());

        // reserves a i32 we can write the document size to later
        doc_output.put_i32(0);

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
            key: Some(key),
            output: &mut self.doc_output,
        })
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.doc_output.put_u8(0x00); // doc terminator

        // writes the total length of the output to the i32 we split off before
        for (i, byte) in (self.doc_output.len() as i32)
            .to_le_bytes()
            .iter()
            .enumerate()
        {
            self.doc_output[i] = *byte;
        }

        self.original_output.unsplit(self.doc_output);

        Ok(())
    }
}
