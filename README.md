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
