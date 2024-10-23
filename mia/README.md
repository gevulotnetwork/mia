# MIA

## Building

The only supported platform for MIA for now is `x86_64-unknown-linux-gnu`.

Install this target, if you don't have it.

```
rustup target add x86_64-unknown-linux-gnu
```

Then just cargo build it. There is a `.cargo/config.toml` that ensures the build is statically linked.

```
cargo build --release
```
