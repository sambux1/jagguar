use ark_ff::Field;

// Multiply a matrix (rows of field elements) by a vector over the same field.
pub fn matrix_vector_multiplication<F: Field>(matrix: &Vec<Vec<F>>, vector: &Vec<F>) -> Vec<F> {
    let mut out = Vec::with_capacity(matrix.len());
    for row in matrix.iter() {
        assert_eq!(row.len(), vector.len(), "row length must match vector length");
        let mut acc = F::ZERO;
        for (a, b) in row.iter().zip(vector.iter()) {
            acc += *a * *b;
        }
        out.push(acc);
    }
    out
}


