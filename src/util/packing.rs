use std::mem;

// Pack a vector of elements into a vector of u64s
pub fn pack_vector<T>(vector: &Vec<T>) -> Vec<u64>
where
    T: Copy + Into<u64>,
{
    let bitwidth = mem::size_of::<T>() * 8;
    assert!(bitwidth <= 64, "Type too large to pack into u64");

    if vector.is_empty() {
        return Vec::new();
    }

    let compression_factor = 64 / bitwidth;
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

// Unpack a vector of u64s into a vector of smaller bitwidth
pub fn unpack_vector<T>(vector: &Vec<u64>) -> Vec<T>
where
    T: Copy + num_traits::FromPrimitive,
{
    let bitwidth = mem::size_of::<T>() * 8;
    assert!(bitwidth <= 64, "Type too large to unpack from u64");

    if vector.is_empty() {
        return Vec::new();
    }

    let compression_factor = 64 / bitwidth;
    let mut out = Vec::with_capacity(vector.len() * compression_factor);
    let mask: u64 = if bitwidth == 64 { u64::MAX } else { (1u64 << bitwidth) - 1 };

    for &word in vector {
        for i in 0..compression_factor {
            let val = (word >> (i * bitwidth)) & mask;
            let t = num_traits::FromPrimitive::from_u64(val)
                .expect("Unpack failed: value does not fit in target type");
            out.push(t);
        }
    }

    out
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // test that the packing and unpacking functions work correctly
    fn test_packing() {
        let input = vec![1u32, 2u32, 3u32, 4u32, 5u32, 6u32];

        // pack the input and check that it is smaller
        let packed = pack_vector(&input);
        assert!(packed.len() < input.len());

        // unpack the input and check that the original size is restored
        let unpacked = unpack_vector(&packed);
        assert!(unpacked.len() == input.len());

        // check that the unpacked input is the same as the original input
        assert_eq!(input, unpacked);
    }
}