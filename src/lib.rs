mod byte;
mod error;
pub mod ser;

pub use error::Error;

use byte::CountingBytes;
use bytes::BytesMut;
use serde::Serialize;

pub fn to_string<T: Serialize>(val: &T, output: &mut BytesMut) -> Result<(), Error> {
    // do a quick pass over the value using our `CountingBytes` impl so we can do
    // one big allocation rather than multiple smaller ones.
    output.reserve(serialised_size_of(val)?);

    val.serialize(ser::Serializer { key: None, output })
}

pub fn serialised_size_of<T: Serialize>(val: &T) -> Result<usize, Error> {
    let mut counting_bytes = CountingBytes::default();
    val.serialize(ser::Serializer {
        key: None,
        output: &mut counting_bytes,
    })?;
    Ok(counting_bytes.bytes)
}

#[cfg(test)]
mod test {
    use super::{serialised_size_of, to_string};
    use bytes::{BufMut, BytesMut};
    use serde::Serialize;

    #[test]
    pub fn test_basic() {
        #[derive(Serialize)]
        pub struct A<'a> {
            cool: i32,
            #[serde(with = "serde_bytes")]
            beans: &'a [u8],
            bro: &'a str,
            b: B<'a>,
        }

        #[derive(Serialize)]
        pub enum Test {
            Abc,
            Def(i32),
            Ghi(i32, i32, i32),
            Jkl { a: i32, b: i32 },
        }

        #[derive(Serialize)]
        pub struct Tup(i32, i32);

        #[derive(Serialize)]
        pub struct B<'a> {
            s: &'a str,
            a: Vec<&'a str>,
            e: Test,
            e2: Test,
            e3: Test,
            e4: Test,
            t: (i32, i32, i32),
            ts: Tup,
        }

        let test = &A {
            cool: 999,
            beans: "so there was this one time at bandcamp".as_bytes(),
            bro: "the craziest thing happened",
            b: B {
                s: "dddd",
                a: vec!["yooo", "mayn"],
                e: Test::Abc,
                e2: Test::Def(1999),
                e3: Test::Ghi(16, 07, 1999),
                e4: Test::Jkl { a: 16, b: 07 },
                t: (16, 07, 1999),
                ts: Tup(99, 100),
            },
        };

        let mut ours = BytesMut::new();
        to_string(&test, &mut ours).unwrap();

        let mut theirs = BytesMut::new().writer();
        bson::ser::to_document(&test)
            .unwrap()
            .to_writer(&mut theirs)
            .unwrap();

        let theirs = theirs.into_inner();
        assert_eq!(ours, theirs);

        let calculated_size = serialised_size_of(&test).unwrap();
        assert_eq!(calculated_size, ours.len());
        assert_eq!(calculated_size, theirs.len());
    }
}
