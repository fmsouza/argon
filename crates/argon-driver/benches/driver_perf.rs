use argon_driver::{CompilationSession, CompileOptions, EmitKind, Pipeline, Target};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn example(path: &str) -> PathBuf {
    repo_root().join("examples").join(path)
}

fn bench_driver(c: &mut Criterion) {
    let subset = example("wasm-subset.arg");
    let modules = example("modules/main.arg");

    let js_options = CompileOptions {
        target: Target::Js,
        pipeline: Pipeline::Ir,
        ..Default::default()
    };
    let wasm_options = CompileOptions {
        target: Target::Wasm,
        pipeline: Pipeline::Ir,
        ..Default::default()
    };
    let native_obj_options = CompileOptions {
        target: Target::Native,
        pipeline: Pipeline::Ir,
        emit: EmitKind::Obj,
        ..Default::default()
    };

    let mut group = c.benchmark_group("driver");

    group.bench_function("check_file", |b| {
        b.iter_batched(
            CompilationSession::new,
            |session| session.check_file(&subset).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("compile_js_file", |b| {
        b.iter_batched(
            CompilationSession::new,
            |session| session.compile_file(&subset, &js_options).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("compile_wasm_file", |b| {
        b.iter_batched(
            CompilationSession::new,
            |session| session.compile_file(&subset, &wasm_options).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("compile_native_obj_file", |b| {
        b.iter_batched(
            CompilationSession::new,
            |session| session.compile_file(&subset, &native_obj_options).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("compile_project_js", |b| {
        b.iter_batched(
            CompilationSession::new,
            |session| session.compile_project(&modules, &js_options).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(driver_perf, bench_driver);
criterion_main!(driver_perf);
