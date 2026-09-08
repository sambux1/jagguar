# Simulator

Jagguar includes a localhost simulator for many-party runs. It can support
up to a few hundred parties. This is not a network deployment.

Requires a recent stable Rust toolchain (the crate uses the 2024 edition).

```bash
cargo simulator
```

This is an alias for:

```bash
cargo run --release --bin simulator --features simulator
```

The example binary runs a localhost OPA aggregation (one server, 25 clients,
a committee) and checks the decoded output against the plaintext sum.

Types and methods are documented in rustdoc (`jagguar::simulator`), included
when you build with `--features simulator`. See the [API reference](api.md).
