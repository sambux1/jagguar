# Jagguar

This is a work-in-progress implementation of single-server secure aggregation.

> [!WARNING]
> This is a work-in-progress implementation designed to run on a single machine. It has not received a security audit.



## Simulator

Requires a recent stable Rust toolchain (the crate uses the 2024 edition).

```bash
cargo simulator
```

This is an alias for:

```bash
cargo run --release --bin simulator --features simulator
```

It runs a localhost OPA aggregation (one server, 25 clients, a committee) and checks the decoded output against the plaintext sum.

Documentation: [Read the Docs](https://jagguar.readthedocs.io/en/latest/)
