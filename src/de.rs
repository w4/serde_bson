use memchr::memchr;
use std::{cell::RefCell, convert::TryInto, fmt::Display};

use serde::{
    de::{
        value::BorrowedStrDeserializer, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
        VariantAccess, Visitor,
    },
    forward_to_deserialize_any, Deserializer,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unexpected map end")]
    UnexpectedMapEnd,
    #[error("unexpected key")]
    UnexpectedKey,
    #[error("end of file")]
    EndOfFile,
    #[error("custom: {0}")]
    Custom(String),
    #[error("malformed map, missing key or document end")]
    MalformedMapMissingKey,
    #[error("unexpected enum")]
    UnexpectedEnum,
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Custom(msg.to_string())
    }
}

thread_local! {
    static ALLOCATOR: RefCell<bumpalo::Bump> = RefCell::new(bumpalo::Bump::new());
}

pub fn from_bytes<'de, D: serde::de::Deserialize<'de>>(data: &'de [u8]) -> Result<D, Error> {
    ALLOCATOR.with_borrow_mut(|allocator| {
        allocator.reset();

        let mut tape = bumpalo::collections::Vec::new_in(allocator);
        to_tape(data, &mut tape);
        D::deserialize(&mut BsonDeserializer { tape: &tape })
    })
}

struct BsonDeserializer<'a, 'de> {
    tape: &'a [Tape<'de>],
}

impl<'a, 'de> BsonDeserializer<'a, 'de> {
    fn next_item(&mut self) -> Option<&'a Tape<'de>> {
        let (next, rest) = self.tape.split_first()?;
        self.tape = rest;
        Some(next)
    }
}

impl<'de> Deserializer<'de> for &mut BsonDeserializer<'_, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.next_item() {
            Some(Tape::DocumentStart) => visitor.visit_map(self),
            Some(Tape::DocumentEnd) => Err(Error::UnexpectedMapEnd),
            Some(Tape::Key(_)) => Err(Error::UnexpectedKey),
            Some(Tape::Double(value)) => visitor.visit_f64(*value),
            Some(Tape::String(value)) => visitor.visit_borrowed_str(value),
            Some(Tape::ArrayStart) => self.deserialize_seq(visitor),
            Some(Tape::Binary(value, _)) => visitor.visit_borrowed_bytes(value),
            Some(Tape::Boolean(value)) => visitor.visit_bool(*value),
            Some(Tape::UtcDateTime(value)) => visitor.visit_i64(*value),
            Some(Tape::Null) => visitor.visit_none(),
            Some(Tape::I32(value)) => visitor.visit_i32(*value),
            Some(Tape::Timestamp(value)) => visitor.visit_u64(*value),
            Some(Tape::I64(value)) => visitor.visit_i64(*value),
            None => Err(Error::EndOfFile),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(Tape::ArrayStart) = self.tape.first() {
            self.tape = &self.tape[1..];
        }

        let res = visitor.visit_seq(&mut *self)?;

        let Some(Tape::DocumentEnd) = self.next_item() else {
            return Err(Error::UnexpectedMapEnd);
        };

        Ok(res)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.next_item() {
            Some(Tape::String(s)) => visitor.visit_enum(s.into_deserializer()),
            Some(Tape::DocumentStart) => {
                let data = visitor.visit_enum(&mut EnumDeserializer { deser: &mut *self })?;

                let Some(Tape::DocumentEnd) = self.next_item() else {
                    return Err(Error::UnexpectedMapEnd);
                };

                Ok(data)
            }
            Some(Tape::ArrayStart) => {
                let data = visitor.visit_enum(&mut EnumDeserializer { deser: &mut *self })?;

                let Some(Tape::DocumentEnd) = self.next_item() else {
                    return Err(Error::UnexpectedMapEnd);
                };

                Ok(data)
            }
            _ => Err(Error::UnexpectedEnum),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct tuple tuple_struct
        map struct identifier ignored_any
    }
}

struct EnumDeserializer<'a, 'b, 'de> {
    deser: &'b mut BsonDeserializer<'a, 'de>,
}

impl<'de> Deserializer<'de> for &mut EnumDeserializer<'_, '_, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(Tape::Key(key)) = self.deser.tape.first() {
            self.deser.tape = &self.deser.tape[1..];
            visitor.visit_borrowed_str(key)
        } else {
            self.deser.deserialize_any(visitor)
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}

impl<'de> VariantAccess<'de> for &mut EnumDeserializer<'_, '_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        unreachable!()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}

impl<'de> EnumAccess<'de> for &mut EnumDeserializer<'_, '_, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let value = seed.deserialize(&mut *self)?;

        Ok((value, self))
    }
}

impl<'de> MapAccess<'de> for BsonDeserializer<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let data = match self.next_item() {
            Some(Tape::DocumentEnd) => return Ok(None),
            Some(Tape::Key(key)) => key,
            _ => return Err(Error::MalformedMapMissingKey),
        };

        seed.deserialize(BorrowedStrDeserializer::new(data))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }
}

impl<'de> SeqAccess<'de> for BsonDeserializer<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if let Some(Tape::DocumentEnd) = self.tape.first() {
            return Ok(None);
        }

        let Some(Tape::Key(_)) = self.next_item() else {
            return Err(Error::MalformedMapMissingKey);
        };

        seed.deserialize(self).map(Some)
    }
}

#[derive(Debug)]
pub enum Tape<'a> {
    DocumentStart,        // start of input or 0x03
    DocumentEnd,          // 0x00
    Key(&'a str),         //
    Double(f64),          // 0x01
    String(&'a str),      // 0x02
    ArrayStart,           // 0x04
    Binary(&'a [u8], u8), // 0x05
    Boolean(bool),        // 0x08
    UtcDateTime(i64),     // 0x09
    Null,                 // 0x0a
    I32(i32),             // 0x10
    Timestamp(u64),       // 0x11
    I64(i64),             // 0x12
}

fn to_tape<'a>(input: &'a [u8], tape: &mut bumpalo::collections::Vec<'_, Tape<'a>>) {
    let length = u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as usize;

    let input = &input[4..length];

    let mut position = 0;
    tape.push(Tape::DocumentStart);

    let take_cstring = |position: &mut usize| {
        let idx = memchr(b'\0', &input[*position..]).expect("unterminated c-string");
        let s = simdutf8::basic::from_utf8(&input[*position..*position + idx]).unwrap();
        *position += idx + 1;
        s
    };

    let take_bytes = |position: &mut usize, n| {
        let res = &input[*position..*position + n];
        *position += n;
        res
    };

    while position < length - 4 {
        position += 1;
        match input[position - 1] {
            0x00 => {
                tape.push(Tape::DocumentEnd);
            }
            0x01 => {
                let key = take_cstring(&mut position);
                let value = f64::from_le_bytes(take_bytes(&mut position, 8).try_into().unwrap());
                tape.push(Tape::Key(key));
                tape.push(Tape::Double(value));
            }
            0x02 => {
                let key = take_cstring(&mut position);
                let length =
                    u32::from_le_bytes(take_bytes(&mut position, 4).try_into().unwrap()) as usize;
                let value =
                    simdutf8::basic::from_utf8(&input[position..position + length - 1]).unwrap();
                position += length;
                tape.push(Tape::Key(key));
                tape.push(Tape::String(value));
            }
            0x03 => {
                let key = take_cstring(&mut position);
                let _length = take_bytes(&mut position, 4);
                tape.push(Tape::Key(key));
                tape.push(Tape::DocumentStart);
            }
            0x04 => {
                let key = take_cstring(&mut position);
                let _length = take_bytes(&mut position, 4);
                tape.push(Tape::Key(key));
                tape.push(Tape::ArrayStart);
            }
            0x05 => {
                let key = take_cstring(&mut position);
                let length =
                    u32::from_le_bytes(take_bytes(&mut position, 4).try_into().unwrap()) as usize;
                let subtype = input[position];
                position += 1;
                let value = &input[position..position + length];
                position += length;
                tape.push(Tape::Key(key));
                tape.push(Tape::Binary(value, subtype));
            }
            0x08 => {
                let key = take_cstring(&mut position);
                let value = input[position] == 1;
                position += 1;
                tape.push(Tape::Key(key));
                tape.push(Tape::Boolean(value));
            }
            0x09 => {
                let key = take_cstring(&mut position);
                let value = i64::from_le_bytes(take_bytes(&mut position, 8).try_into().unwrap());
                tape.push(Tape::Key(key));
                tape.push(Tape::UtcDateTime(value));
            }
            0x0a => {
                let key = take_cstring(&mut position);
                tape.push(Tape::Key(key));
                tape.push(Tape::Null);
            }
            0x10 => {
                let key = take_cstring(&mut position);
                let value = i32::from_le_bytes(take_bytes(&mut position, 4).try_into().unwrap());
                tape.push(Tape::Key(key));
                tape.push(Tape::I32(value));
            }
            0x11 => {
                let key = take_cstring(&mut position);
                let value = u64::from_le_bytes(take_bytes(&mut position, 8).try_into().unwrap());
                tape.push(Tape::Key(key));
                tape.push(Tape::Timestamp(value));
            }
            0x12 => {
                let key = take_cstring(&mut position);
                let value = i64::from_le_bytes(take_bytes(&mut position, 8).try_into().unwrap());
                tape.push(Tape::Key(key));
                tape.push(Tape::I64(value));
            }
            _ => {}
        };
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn deserialize() {
        let f = std::fs::read("test/test.bin").unwrap();

        let bump = bumpalo::Bump::new();
        let mut tape = bumpalo::collections::Vec::new_in(&bump);

        super::to_tape(&f, &mut tape);
        insta::assert_debug_snapshot!(tape);
    }
}
