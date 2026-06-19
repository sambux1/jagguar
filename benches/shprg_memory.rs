#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use jagguar::crypto::SeedHomomorphicPRG;

const ONE_MIB: u64 = 1024 * 1024;

// Each output element carries `bits_per_element` bits of pseudorandomness (92 by default).
// Round up so we never undershoot the target byte volume.
fn elements_for_bytes(target_bytes: u64, bits_per_element: u32) -> usize {
    let target_bits = target_bytes.saturating_mul(8);
    let bits_per_element = u64::from(bits_per_element);
    ((target_bits + bits_per_element - 1) / bits_per_element) as usize
}

fn main() {

    let targets: Vec<(&str, u64)> = vec![
        ("100_KiB", 100 * 1024),
        ("1_MiB", ONE_MIB),
        ("10_MiB", 10 * ONE_MIB),
    ];

    let prg = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
    let bits_per_element = prg.inner_modulus_bits();

    eprintln!(
        "SHPRG peak heap memory ({bits_per_element} bits per output element)\n"
    );

    for (label, bytes) in targets {
        let n = elements_for_bytes(bytes, bits_per_element);
        eprintln!("--- {label} (target {bytes} bytes of randomness, {n} elements) ---");
        {
            let _profiler = dhat::Profiler::new_heap();
            let output = prg.expand(n);
            std::hint::black_box(&output);
        }
        eprintln!();
    }
}
