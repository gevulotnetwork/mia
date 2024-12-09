# MIA

MIA is a Minimal Init Application. It serves as init process (an alternative to systemd) in Linux VM built for Gevulot Network.
It is used by default in [Gevulot Control CLI](https://github.com/gevulotnetwork/gvltctl) tool.
Its job is to configure environment and launch the main application in the VM.

## Repository

- `mia` - MIA source code
- `mia-installer` - installer for MIA (CLI and library)

## Docs

MIA is using Gevulot Runtime configuration to configure the environment for main application.

See docs of [`gevulot_rs::runtime_config`](https://docs.rs/gevulot-rs/latest/gevulot_rs/runtime_config/index.html).
