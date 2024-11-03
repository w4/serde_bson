use bytes::BufMut;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[derive(serde::Serialize)]
pub struct A {
    a: String,
    b: String,
    c: String,
    d: i64,
    e: f64,
    #[serde(with = "serde_bytes")]
    f: Vec<u8>,
}

fn benchmark(c: &mut Criterion) {
    let val = A {
        a: "Now this is a story all about how
            My life got flipped turned upside down
            And I'd like to take a minute, just sit right there
            I'll tell you how I became the prince of a town called Bel-Air"
            .to_string(),
        b: "In West Philadelphia born and raised
            On the playground is where I spent most of my days
            Chillin' out, maxin', relaxin' all cool
            And all shootin' some b-ball outside of the school
            When a couple of guys who were up to no good
            Started makin' trouble in my neighborhood"
            .to_string(),
        c: "I got in one little fight and my mom got scared
            And said 'You're movin' with your auntie and uncle in Bel-Air'"
            .to_string(),
        d: 420,
        e: 420.69696969696969,
        f: "Above are some popular 'pop culture' references for your perusal and enjoyment".into(),
    };

    c.bench_function("serialize: mongodb's bson", |b| {
        let mut theirs = Vec::new();

        b.iter(|| {
            bson::ser::to_document(black_box(&val))
                .unwrap()
                .to_writer(&mut theirs)
                .unwrap();
            theirs.clear();
        })
    });

    c.bench_function("serialize: serde_bson", |b| {
        let mut out = bytes::BytesMut::new();

        b.iter(|| {
            serde_bson::to_string(black_box(&val), &mut out).unwrap();
            drop(out.split());
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
