mod error;
pub mod ser;

pub use error::Error;

use bytes::BytesMut;
use serde::Serialize;

pub fn to_string<T: Serialize>(val: &T, output: &mut BytesMut) -> Result<(), Error> {
    val.serialize(ser::Serializer { key: None, output })
}

#[cfg(test)]
mod test {
    use super::to_string;
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
        pub struct B<'a> {
            s: &'a str,
        }

        let test = &A {
            cool: 999,
            beans: "so there was this one time at bandcamp".as_bytes(),
            bro: "the craziest thing happened",
            b: B { s: "dddd" },
        };

        let mut ours = BytesMut::new();
        to_string(&test, &mut ours).unwrap();

        let mut theirs = BytesMut::new().writer();
        bson::ser::to_document(&test)
            .unwrap()
            .to_writer(&mut theirs)
            .unwrap();

        assert_eq!(ours, theirs.into_inner());
    }
}
