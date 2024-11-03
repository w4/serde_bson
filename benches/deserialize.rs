use bytes::BufMut;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct A<'a> {
    cool: i32,
    #[serde(with = "serde_bytes")]
    beans: &'a [u8],
    bro: &'a str,
    b: B<'a>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Test {
    Abc,
    Def(i32),
    Ghi(i32, i32, i32),
    Jkl { a: i32, b: i32 },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Tup(i32, i32);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct B<'a> {
    s: &'a str,
    a: Vec<&'a str>,
    e: Test,
    e2: Test,
    e3: Test,
    e4: Test,
    t: (i32, i32, i32),
    ts: Tup,
    y: bool,
}

fn benchmark(c: &mut Criterion) {
    let data = include_bytes!("../test/test.bin");

    c.bench_function("deserialize: mongodb's bson", |b| {
        b.iter(|| bson::de::from_slice::<A>(black_box(data)))
    });

    c.bench_function("deserialize: serde_bson", |b| {
        b.iter(|| serde_bson::de::from_bytes::<A>(black_box(data)));
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
