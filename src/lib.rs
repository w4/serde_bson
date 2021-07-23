mod error;
pub mod ser;

pub use error::Error;

use bytes::{BufMut, BytesMut};
use serde::Serialize;

pub fn to_string<T: Serialize>(val: &T, output: &mut BytesMut) -> Result<(), Error> {
    const SIZE_OF_SIZE: usize = std::mem::size_of::<i32>();

    // essentially reserves a i32 we can prepend back onto the BytesMut later
    // at the cost of an atomic increment
    output.put_i32(0);
    let mut size = output.split_to(SIZE_OF_SIZE);

    val.serialize(ser::Serializer { key: None, output })?;

    // writes the total length of the output to the i32 we split off before
    for (i, byte) in ((output.len() + SIZE_OF_SIZE) as i32)
        .to_le_bytes()
        .iter()
        .enumerate()
    {
        size[i] = *byte;
    }

    // this is safe because `unsplit` can't panic
    take_mut::take(output, move |output| {
        // O(1) prepend since `size` originally came from `output`.
        size.unsplit(output);
        size
    });

    Ok(())
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
        }

        let test = &A {
            cool: 999,
            beans: "so there was this one time at bandcamp".as_bytes(),
            bro: "the craziest thing happened",
        };

        let mut ours = BytesMut::new();
        to_string(&test, &mut ours);

        let mut theirs = BytesMut::new().writer();
        bson::ser::to_document(&test)
            .unwrap()
            .to_writer(&mut theirs)
            .unwrap();

        assert_eq!(ours, theirs.into_inner());
    }
}
