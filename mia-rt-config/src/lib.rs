//! MIA runtime config.
//!
//! MIA uses this config to setup environment before launching main application.
//!
//! `follow-config` field allows to chain multiple configs. It contains path to the
//! next config to process after current one is finished.
//!
//! MIA will perform the following actions for every config:
//!
//! 1. Mount defaults (if they are not mounted yet)
//! 2. Mount specifies directories in the order of specification in `mounts`
//! 3. Set environment variables for the current process from `env`
//! 4. Set working directory for the current process from `working-dir`
//! 5. Load kernel modules using `modprobe` in order of specification in `kernel-modules`
//! 6. Run boot commands from `bootcmd`
//!
//! If current config defines a `command` to run, it will be updated with its arguments.
//! Finally, if there is a following config, it will be loaded and handled in the same way.
//!
//! Because loading following config happens after mounting and boot commands, it may be
//! taken from mounted directory or generated with boot commands.

#[cfg(feature = "v1")]
#[path = "v1/mod.rs"]
mod implementation;

pub use implementation::*;
