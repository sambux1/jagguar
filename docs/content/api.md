# API reference

These pages do not specify the crate API. Rustdoc is the specification:
modules, types, invariants, and paper citations.

**On Read the Docs**, each build publishes rustdoc here:

[jagguar rustdoc](../rustdoc/jagguar/index.html)

**Locally:**

```bash
cargo doc --no-deps --features simulator --open
```

`--features simulator` includes the localhost simulator module. Omit it if
you only need the library surface.
