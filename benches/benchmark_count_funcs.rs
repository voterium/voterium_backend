use criterion::{black_box, criterion_group, criterion_main, Criterion};

use voterium_backend::counting_funcs::{
    count_votes_1, count_votes_2, count_votes_3, count_votes_4, count_votes_5,
    count_votes_6, count_votes_7, count_votes_8, count_votes_9, count_votes_10,
    count_votes_11, count_votes_12, count_votes_13, count_votes_14, count_votes_15,
    count_votes_16, count_votes_17, count_votes_18, count_votes_19, count_votes_20,
    count_votes_21, count_votes_22
};

use voterium_backend::models::{Choice, Config};
use voterium_backend::utils::load_voting_config;

fn make_input() -> Vec<Choice> {
    let config: Config = load_voting_config();
    config.choices
}


fn benchmark_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("Function Versions");
    let input = make_input();

    // group.bench_function("count_votes_1", |b| {
    //     b.iter(|| count_votes_1())
    // });

    // group.bench_function("count_votes_2", |b| {
    //     b.iter(|| count_votes_2())
    // });

    // group.bench_function("count_votes_3", |b| {
    //     b.iter(|| count_votes_3())
    // });

    // group.bench_function("count_votes_4", |b| {
    //     b.iter(|| count_votes_4())
    // });

    // group.bench_function("count_votes_5", |b| {
    //     b.iter(|| count_votes_5())
    // });

    // group.bench_function("count_votes_6", |b| {
    //     b.iter(|| count_votes_6())
    // });

    // group.bench_function("count_votes_7", |b| {
    //     b.iter(|| count_votes_7())
    // });

    // group.bench_function("count_votes_8", |b| {
    //     b.iter(|| count_votes_8())
    // });

    // group.bench_function("count_votes_9", |b| {
    //     b.iter(|| count_votes_9())
    // });

    // group.bench_function("count_votes_10", |b| {
    //     b.iter(|| count_votes_10(black_box(&input)))
    // });

    // group.bench_function("count_votes_11", |b| {
    //     b.iter(|| count_votes_11(black_box(&input)))
    // });

    // group.bench_function("count_votes_12", |b| {
    //     b.iter(|| count_votes_12(black_box(&input)))
    // });

    // group.bench_function("count_votes_13", |b| {
    //     b.iter(|| count_votes_13(black_box(&input)))
    // });

    // group.bench_function("count_votes_14", |b| {
    //     b.iter(|| count_votes_14(black_box(&input)))
    // });

    // group.bench_function("count_votes_15", |b| {
    //     b.iter(|| count_votes_15(black_box(&input)))
    // });

    // group.bench_function("count_votes_16", |b| {
    //     b.iter(|| count_votes_16(black_box(&input)))
    // });

    // group.bench_function("count_votes_17", |b| {
    //     b.iter(|| count_votes_17(black_box(&input)))
    // });

    group.bench_function("count_votes_18", |b| {
        b.iter(|| count_votes_18(black_box(&input)))
    });

    group.bench_function("count_votes_19", |b| {
        b.iter(|| count_votes_19(black_box(&input)))
    });

    group.bench_function("count_votes_20", |b| {
        b.iter(|| count_votes_20(black_box(&input)))
    });

    group.bench_function("count_votes_21", |b| {
        b.iter(|| count_votes_21(black_box(&input)))
    });

    group.bench_function("count_votes_22", |b| {
        b.iter(|| count_votes_22(black_box(&input)))
    });

    group.finish();
}

criterion_group!(benches, benchmark_functions);
criterion_main!(benches);
