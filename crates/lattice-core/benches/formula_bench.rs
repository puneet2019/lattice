//! Benchmarks for formula evaluation at various scales.

use criterion::{Criterion, criterion_group, criterion_main};

use lattice_core::cell::CellValue;
use lattice_core::formula::FormulaEngine;
use lattice_core::formula::evaluator::SimpleEvaluator;
use lattice_core::sheet::Sheet;

/// Create a sheet pre-populated with numeric values in column A.
fn make_numeric_sheet(rows: u32) -> Sheet {
    let mut sheet = Sheet::new("Bench");
    for r in 0..rows {
        sheet.set_value(r, 0, CellValue::Number((r + 1) as f64));
    }
    sheet
}

fn bench_sum_1k(c: &mut Criterion) {
    let sheet = make_numeric_sheet(1_000);
    let engine = SimpleEvaluator;
    c.bench_function("SUM 1K cells", |b| {
        b.iter(|| {
            let result = engine.evaluate("SUM(A1:A1000)", &sheet);
            assert!(result.is_ok());
        })
    });
}

fn bench_sum_10k(c: &mut Criterion) {
    let sheet = make_numeric_sheet(10_000);
    let engine = SimpleEvaluator;
    c.bench_function("SUM 10K cells", |b| {
        b.iter(|| {
            let result = engine.evaluate("SUM(A1:A10000)", &sheet);
            assert!(result.is_ok());
        })
    });
}

fn bench_sum_100k(c: &mut Criterion) {
    let sheet = make_numeric_sheet(100_000);
    let engine = SimpleEvaluator;
    c.bench_function("SUM 100K cells", |b| {
        b.iter(|| {
            let result = engine.evaluate("SUM(A1:A100000)", &sheet);
            assert!(result.is_ok());
        })
    });
}

fn bench_average_10k(c: &mut Criterion) {
    let sheet = make_numeric_sheet(10_000);
    let engine = SimpleEvaluator;
    c.bench_function("AVERAGE 10K cells", |b| {
        b.iter(|| {
            let result = engine.evaluate("AVERAGE(A1:A10000)", &sheet);
            assert!(result.is_ok());
        })
    });
}

fn bench_count_10k(c: &mut Criterion) {
    let sheet = make_numeric_sheet(10_000);
    let engine = SimpleEvaluator;
    c.bench_function("COUNT 10K cells", |b| {
        b.iter(|| {
            let result = engine.evaluate("COUNT(A1:A10000)", &sheet);
            assert!(result.is_ok());
        })
    });
}

fn bench_simple_arithmetic(c: &mut Criterion) {
    let sheet = make_numeric_sheet(100);
    let engine = SimpleEvaluator;
    c.bench_function("arithmetic A1+A2*A3-A4/A5", |b| {
        b.iter(|| {
            let result = engine.evaluate("A1+A2*A3-A4/A5", &sheet);
            assert!(result.is_ok());
        })
    });
}

criterion_group!(
    formula_benches,
    bench_sum_1k,
    bench_sum_10k,
    bench_sum_100k,
    bench_average_10k,
    bench_count_10k,
    bench_simple_arithmetic,
);
criterion_main!(formula_benches);
