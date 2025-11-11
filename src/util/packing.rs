use std::mem;

// Pack a vector of elements into a vector of u64s
pub fn pack_vector<T>(vector: &Vec<T>, target_bitwidth: usize) -> Vec<u64>
where
    T: Copy + Into<u64>,
{
    let bitwidth = mem::size_of::<T>() * 8;
    assert!(bitwidth <= target_bitwidth, "Type too large to pack into u64");

    if vector.is_empty() {
        return Vec::new();
    }

    let compression_factor = target_bitwidth / bitwidth;
    let mut out = Vec::with_capacity((vector.len() + compression_factor - 1) / compression_factor);

    let mut acc = 0u64;
    let mut count = 0;

    for &v in vector {
        acc |= (v.into() & ((1u64 << bitwidth) - 1)) << (count * bitwidth);
        count += 1;
        if count == compression_factor {
            out.push(acc);
            acc = 0;
            count = 0;
        }
    }

    if count > 0 {
        out.push(acc);
    }

    out
}