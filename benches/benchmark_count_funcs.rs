use criterion::{black_box, criterion_group, criterion_main, Criterion};

use voterium_backend::counting::counting_funcs::{
    count_votes_01, count_votes_03, count_votes_04, count_votes_06, count_votes_08, count_votes_10,
    count_votes_11, count_votes_12, count_votes_13, count_votes_14, count_votes_15, count_votes_16,
    count_votes_18, count_votes_19, count_votes_20, count_votes_22, count_votes_23, count_votes_24,
    count_votes_25, count_votes_26, count_votes_27, count_votes_28, count_votes_29, count_votes_30,
    count_votes_31, count_votes_34, count_votes_35,
};

use voterium_backend::ledgers::load_cl;
use voterium_backend::utils::load_voting_config;

fn benchmark_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("Function Versions");
    let config = load_voting_config("examples/voting_config_012.json");
    let choices = config.choices;   
    let data = load_cl("examples/cl_1M.csv").unwrap();

    // group.bench_function("count_votes_01", |b| b.iter(|| count_votes_01(&data)));

    // // group.bench_function("count_votes_2", |b| {
    // //     b.iter(|| count_votes_02(&data))
    // // });

    // group.bench_function("count_votes_03", |b| b.iter(|| count_votes_03(&data)));

    // group.bench_function("count_votes_04", |b| b.iter(|| count_votes_04(&data)));

    // // group.bench_function("count_votes_5", |b| {
    // //     b.iter(|| count_votes_05(&data))
    // // });

    // group.bench_function("count_votes_06", |b| b.iter(|| count_votes_06(&data)));

    // // group.bench_function("count_votes_7", |b| {
    // //     b.iter(|| count_votes_07(&data))
    // // });

    // group.bench_function("count_votes_08", |b| b.iter(|| count_votes_08(&data)));

    // // group.bench_function("count_votes_9", |b| {
    // //     b.iter(|| count_votes_09(&data))
    // // });

    // group.bench_function("count_votes_10", |b| {
    //     b.iter(|| count_votes_10(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_11", |b| {
    //     b.iter(|| count_votes_11(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_12", |b| {
    //     b.iter(|| count_votes_12(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_13", |b| {
    //     b.iter(|| count_votes_13(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_14", |b| {
    //     b.iter(|| count_votes_14(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_15", |b| {
    //     b.iter(|| count_votes_15(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_16", |b| {
    //     b.iter(|| count_votes_16(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_17", |b| {
    //     b.iter(|| count_votes_17(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_18", |b| {
    //     b.iter(|| count_votes_18(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_19", |b| {
    //     b.iter(|| count_votes_19(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_20", |b| {
    //     b.iter(|| count_votes_20(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_21", |b| {
    //     b.iter(|| count_votes_21(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_22", |b| {
    //     b.iter(|| count_votes_22(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_23", |b| {
    //     b.iter(|| count_votes_23(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_24", |b| {
    //     b.iter(|| count_votes_24(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_25", |b| {
    //     b.iter(|| count_votes_25(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_26", |b| {
    //     b.iter(|| count_votes_26(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_27", |b| {
    //     b.iter(|| count_votes_27(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_28", |b| {
    //     b.iter(|| count_votes_28(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_29", |b| {
    //     b.iter(|| count_votes_29(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_30", |b| {
    //     b.iter(|| count_votes_30(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_31", |b| {
    //     b.iter(|| count_votes_31(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_32", |b| {
    //     b.iter(|| count_votes_32(black_box(&data), black_box(&input)))
    // });

    // group.bench_function("count_votes_33", |b| {
    //     b.iter(|| count_votes_33(black_box(&data), black_box(&input)))
    // });

    group.bench_function("count_votes_34", |b| {
        b.iter(|| count_votes_34(black_box(&data), black_box(&choices)))
    });

    group.bench_function("count_votes_35", |b| {
        b.iter(|| count_votes_35(black_box(&data), black_box(&choices)))
    });

    group.finish();
}

criterion_group!(benches, benchmark_functions);
criterion_main!(benches);
