---
name: benchmark
description: Run performance benchmarks — formula recalculation, file I/O, grid rendering
---

# /benchmark

Run Lattice performance benchmarks.

## Steps

1. Run Rust benchmarks:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo bench --workspace 2>&1
   ```

2. Report results against targets:

   | Operation | Target | Actual |
   |-----------|--------|--------|
   | Cell edit → recalc (10 deps) | <1ms | ? |
   | Recalc 10k formula cells | <100ms | ? |
   | Recalc 100k formula cells | <1s | ? |
   | Open 10MB .xlsx | <2s | ? |
   | Save 10MB .xlsx | <2s | ? |
   | MCP read_cell round-trip | <5ms | ? |
   | MCP write_cell round-trip | <10ms | ? |

3. Flag any regression from previous run.

## Benchmark Code Location
- `crates/lattice-core/benches/` — Engine benchmarks
- `crates/lattice-io/benches/` — File I/O benchmarks
- `crates/lattice-mcp/benches/` — MCP latency benchmarks

## How to Add a Benchmark
```rust
// In benches/engine_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_recalc_10k(c: &mut Criterion) {
    let wb = create_workbook_with_formulas(10_000);
    c.bench_function("recalc_10k_formulas", |b| {
        b.iter(|| wb.recalculate_all())
    });
}

criterion_group!(benches, bench_recalc_10k);
criterion_main!(benches);
```
