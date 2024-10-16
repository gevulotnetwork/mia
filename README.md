MIA
===

MIA is a Minimal Init Application. It serves as entrypoint for VM images built from containers by `gvltctl`.

## Functionality

MIA first mounts any necessary filesystems, then starts the user's init process or directly the application.

MIA is typically called by the kernel when booting a VM. It gets passed any init options that the user specified.

Example invocation:
```
/sbin/mia \
    --module nvidia \
    --mount proc:/proc:proc: \
    --mount input:/mnt/input:9p:trans=virtio,version=9p2000.L \
    /bin/bash -c "echo hello"
```

The mount syntax is `<device-or-virtfs-tag>:<mountpoint>:<fs-type>:<options>`.

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
