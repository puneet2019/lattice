//! Benchmarks for cell read/write operations at scale.

use criterion::{Criterion, criterion_group, criterion_main};

use lattice_core::cell::CellValue;
use lattice_core::sheet::Sheet;
use lattice_core::workbook::Workbook;

fn bench_write_1k_cells(c: &mut Criterion) {
    c.bench_function("write 1K cells", |b| {
        b.iter(|| {
            let mut sheet = Sheet::new("Bench");
            for r in 0..1_000u32 {
                sheet.set_value(r, 0, CellValue::Number(r as f64));
            }
        })
    });
}

fn bench_write_10k_cells(c: &mut Criterion) {
    c.bench_function("write 10K cells", |b| {
        b.iter(|| {
            let mut sheet = Sheet::new("Bench");
            for r in 0..10_000u32 {
                sheet.set_value(r, 0, CellValue::Number(r as f64));
            }
        })
    });
}

fn bench_write_100k_cells(c: &mut Criterion) {
    c.bench_function("write 100K cells", |b| {
        b.iter(|| {
            let mut sheet = Sheet::new("Bench");
            for r in 0..100_000u32 {
                sheet.set_value(r, 0, CellValue::Number(r as f64));
            }
        })
    });
}

fn bench_read_10k_cells(c: &mut Criterion) {
    let mut sheet = Sheet::new("Bench");
    for r in 0..10_000u32 {
        sheet.set_value(r, 0, CellValue::Number(r as f64));
    }
    c.bench_function("read 10K cells", |b| {
        b.iter(|| {
            for r in 0..10_000u32 {
                let _ = sheet.get_cell(r, 0);
            }
        })
    });
}

fn bench_read_100k_cells(c: &mut Criterion) {
    let mut sheet = Sheet::new("Bench");
    for r in 0..100_000u32 {
        sheet.set_value(r, 0, CellValue::Number(r as f64));
    }
    c.bench_function("read 100K cells", |b| {
        b.iter(|| {
            for r in 0..100_000u32 {
                let _ = sheet.get_cell(r, 0);
            }
        })
    });
}

fn bench_workbook_set_cell_10k(c: &mut Criterion) {
    c.bench_function("workbook set_cell 10K", |b| {
        b.iter(|| {
            let mut wb = Workbook::new();
            for r in 0..10_000u32 {
                wb.set_cell("Sheet1", r, 0, CellValue::Number(r as f64))
                    .unwrap();
            }
        })
    });
}

fn bench_used_range_100k(c: &mut Criterion) {
    let mut sheet = Sheet::new("Bench");
    for r in 0..100_000u32 {
        sheet.set_value(r, 0, CellValue::Number(r as f64));
    }
    c.bench_function("used_range on 100K cells", |b| {
        b.iter(|| {
            let _ = sheet.used_range();
        })
    });
}

fn bench_clear_range_1k(c: &mut Criterion) {
    c.bench_function("clear 1K cells", |b| {
        b.iter_batched(
            || {
                let mut sheet = Sheet::new("Bench");
                for r in 0..1_000u32 {
                    sheet.set_value(r, 0, CellValue::Number(r as f64));
                }
                sheet
            },
            |mut sheet| {
                for r in 0..1_000u32 {
                    sheet.clear_cell(r, 0);
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    cell_ops_benches,
    bench_write_1k_cells,
    bench_write_10k_cells,
    bench_write_100k_cells,
    bench_read_10k_cells,
    bench_read_100k_cells,
    bench_workbook_set_cell_10k,
    bench_used_range_100k,
    bench_clear_range_1k,
);
criterion_main!(cell_ops_benches);
