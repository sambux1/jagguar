use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use jagguar::crypto::SeedHomomorphicPRG;

const ONE_MIB: u64 = 1024 * 1024;

// Each output element carries `bits_per_element` bits of pseudorandomness (92 by default).
// Round up so we never undershoot the target byte volume.
fn elements_for_bytes(target_bytes: u64, bits_per_element: u32) -> usize {
    let target_bits = target_bytes.saturating_mul(8);
    let bits_per_element = u64::from(bits_per_element);
    ((target_bits + bits_per_element - 1) / bits_per_element) as usize
}

fn benchmark_shprg_expand(c: &mut Criterion) {
    let prg = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
    let bits_per_element = prg.inner_modulus_bits();

    let mut group = c.benchmark_group("shprg_expand");
    group.sample_size(10);

    for (label, bytes) in [("64_KiB", 64 * 1024), ("1_MiB", ONE_MIB)] {
        let n = elements_for_bytes(bytes, bits_per_element);
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::new("expand", label), &n, |b, &n| {
            b.iter(|| black_box(prg.expand(n)));
        });
    }

    // Full GiB runs are expensive; allow more wall-clock time per sample.
    group.measurement_time(Duration::from_secs(20));
    group.finish();
}

criterion_group!(benches, benchmark_shprg_expand);
criterion_main!(benches);
