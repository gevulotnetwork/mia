MIA
===

MIA is a Minimal Init Application. It serves as init process (an alternative to systemd) in Linux VM built for Gevulot Network.
It is used by default in [Gevulot Control CLI](https://github.com/gevulotnetwork/gvltctl) tool.
Its job is to configure environment and launch the main application in the VM.

## Repository

- `mia` - MIA source code
- `mia-installer` - installer for MIA (CLI and library)
- `mia-rt-config` - MIA Runtime Configuration

## Docs

MIA is configured through runtime configuration files `config.yaml`.

To understand how MIA operates, check out `mia-rt-config` docs:

```
cargo doc -p mia-rt-config
```
