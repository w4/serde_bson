use bytes::{BufMut, BytesMut};

pub trait BytesLikeBuf {
    type Out: BytesLikeBuf;

    fn put_u8(&mut self, v: u8);
    fn put_i32_le(&mut self, v: i32);
    fn put_i64_le(&mut self, v: i64);
    fn put_f64_le(&mut self, v: f64);
    fn put_slice(&mut self, s: &[u8]);
    fn split_off(&mut self, at: usize) -> Self::Out;
    fn unsplit(&mut self, other: Self::Out);
    fn len(&mut self) -> usize;
    fn byte_mut(&mut self, at: usize) -> &mut u8;
}

macro_rules! deref_impl {
    (
        impl $trait:ident for $ty:ident {
            $(fn $func:ident(&mut self, $($param_name:ident$(: $param_ty:ty)?),*)$( -> $ret:ty)?$( where Self: $deref:ident)?;)*
        }
    ) => {
        impl $trait for $ty {
            type Out = $ty;

            $(
                fn $func(&mut self, $($param_name$(: $param_ty)?,)*)$( -> $ret)? {
                    <Self$( as $deref)?>::$func(self, $($param_name,)*)
                }
            )*

            fn byte_mut(&mut self, at: usize) -> &mut u8 {
                &mut self[at]
            }
        }
    };
}

deref_impl!(
    impl BytesLikeBuf for BytesMut {
        fn put_u8(&mut self, v: u8) where Self: BufMut;
        fn put_i32_le(&mut self, v: i32) where Self: BufMut;
        fn put_i64_le(&mut self, v: i64) where Self: BufMut;
        fn put_f64_le(&mut self, v: f64) where Self: BufMut;
        fn put_slice(&mut self, s: &[u8]) where Self: BufMut;
        fn split_off(&mut self, at: usize) -> BytesMut;
        fn unsplit(&mut self, other: Self);
        fn len(&mut self,) -> usize;
    }
);

impl<B: BytesLikeBuf> BytesLikeBuf for &mut B {
    type Out = <B as BytesLikeBuf>::Out;

    fn put_u8(&mut self, v: u8) {
        B::put_u8(self, v)
    }

    fn put_i32_le(&mut self, v: i32) {
        B::put_i32_le(self, v)
    }

    fn put_i64_le(&mut self, v: i64) {
        B::put_i64_le(self, v)
    }

    fn put_f64_le(&mut self, v: f64) {
        B::put_f64_le(self, v)
    }

    fn put_slice(&mut self, s: &[u8]) {
        B::put_slice(self, s)
    }

    fn split_off(&mut self, at: usize) -> Self::Out {
        B::split_off(self, at)
    }

    fn unsplit(&mut self, other: Self::Out) {
        B::unsplit(self, other)
    }

    fn len(&mut self) -> usize {
        B::len(self)
    }

    fn byte_mut(&mut self, at: usize) -> &mut u8 {
        B::byte_mut(self, at)
    }
}

#[derive(Default)]
pub struct CountingBytes {
    pub bytes: usize,
    fake_byte: u8,
}

impl BytesLikeBuf for CountingBytes {
    type Out = CountingBytes;

    fn put_u8(&mut self, _v: u8) {
        self.bytes += std::mem::size_of::<u8>();
    }

    fn put_i32_le(&mut self, _v: i32) {
        self.bytes += std::mem::size_of::<i32>();
    }

    fn put_i64_le(&mut self, _v: i64) {
        self.bytes += std::mem::size_of::<i64>();
    }

    fn put_f64_le(&mut self, _v: f64) {
        self.bytes += std::mem::size_of::<f64>();
    }

    fn put_slice(&mut self, s: &[u8]) {
        self.bytes += std::mem::size_of_val(s);
    }

    fn split_off(&mut self, _at: usize) -> Self {
        CountingBytes {
            bytes: 0,
            fake_byte: 0,
        }
    }

    fn unsplit(&mut self, other: Self) {
        self.bytes += other.bytes;
    }

    fn len(&mut self) -> usize {
        self.bytes
    }

    fn byte_mut(&mut self, _at: usize) -> &mut u8 {
        self.fake_byte = 0;
        &mut self.fake_byte
    }
}
