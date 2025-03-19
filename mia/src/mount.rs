use std::path::PathBuf;
use std::{fmt, fs};

use gevulot_rs::runtime_config::Mount as RuntimeMount;

pub use nix::mount::MsFlags;

const TARGET: &str = "mount";

/// Mount description structure.
#[derive(Debug, Clone)]
pub struct Mount {
    /// Source or label of the filesystem to mount.
    pub source: Option<String>,

    /// Target path where to mount the filesystem.
    pub target: PathBuf,

    /// Type of the filesystem to mount.
    pub fstype: Option<String>,

    /// Mount flags.
    pub flags: MsFlags,

    /// Mount options.
    ///
    /// Interpreted by filesystem. See `mount(8)` for available options.
    pub options: Option<String>,

    /// If set to `false` mount failure will not interrupt the execution.
    pub required: bool,
}

impl fmt::Display for Mount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}{}",
            self.source.as_deref().unwrap_or(""),
            self.target.display(),
            self.fstype.as_deref().unwrap_or(""),
            self.options.as_deref().unwrap_or(""),
            if self.required { " [required]" } else { "" },
        )
    }
}

impl Mount {
    pub fn mount(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!(target: TARGET, "{}", self);

        let inner = || -> Result<(), Box<dyn std::error::Error>> {
            if !self.target.exists() {
                fs::create_dir_all(&self.target)?;
            }
            nix::mount::mount(
                self.source.as_deref(),
                &self.target,
                self.fstype.as_deref(),
                self.flags,
                self.options.as_deref(),
            )?;
            Ok(())
        };
        let result = inner();

        if let Err(err) = result {
            if self.required {
                return Err(err);
            } else {
                log::warn!(target: TARGET, "{}", err);
            }
        }

        Ok(())
    }
}

impl TryFrom<&RuntimeMount> for Mount {
    type Error = &'static str;

    fn try_from(value: &RuntimeMount) -> Result<Self, &'static str> {
        Ok(Self {
            source: Some(value.source.clone()),
            target: PathBuf::from(value.target.clone()),
            fstype: value.fstype.clone(),
            flags: if let Some(bits) = value.flags {
                MsFlags::from_bits(bits).ok_or("invalid mount flags")?
            } else {
                MsFlags::empty()
            },
            options: value.data.clone(),
            required: true, // All user mounts are considered required
        })
    }
}

type ConstSource = &'static str;
type ConstTarget = &'static str;
type ConstFsType = &'static str;
type ConstOptions = Option<&'static str>;
type ConstMount = (ConstSource, ConstTarget, ConstFsType, MsFlags, ConstOptions);

impl From<&ConstMount> for Mount {
    fn from(value: &ConstMount) -> Self {
        Mount {
            source: Some(value.0.to_string()),
            target: PathBuf::from(value.1),
            fstype: Some(value.2.to_string()),
            flags: value.3,
            options: value.4.map(ToString::to_string),
            required: false, // All default mounts are considered not required,
                             // because they depend on kernel config
        }
    }
}

/// Table of default mounts performed by MIA.
/// This includes Kernel API mounts.
///
/// Each entry is goint to be converted into [`Mount`].
///
/// Based on systemd:
/// https://github.com/systemd/systemd/blob/v257.4/src/shared/mount-setup.c#L79-L120
pub const DEFAULT_MOUNT_TABLE: &[ConstMount] = &[
    (
        "proc",
        "/proc",
        "proc",
        MsFlags::MS_NOSUID
            .union(MsFlags::MS_NOEXEC)
            .union(MsFlags::MS_NODEV),
        None,
    ),
    (
        "sysfs",
        "/sys",
        "sysfs",
        MsFlags::MS_NOSUID
            .union(MsFlags::MS_NOEXEC)
            .union(MsFlags::MS_NODEV),
        None,
    ),
    (
        // Right now we build kernel with CONFIG_DEVTMPFS_MOUNT=y, so we don't need to mount /dev.
        // It will be automatically mounted by kernel. That's why we set this to not critical.
        // This mount will result into error: "EBUSY: Device or resource busy", which is fine.
        "devtmpfs",
        "/dev",
        "devtmpfs",
        MsFlags::MS_NOSUID.union(MsFlags::MS_STRICTATIME),
        Some("mode=0755,size=4m"),
    ),
    (
        "tmpfs",
        "/dev/shm",
        "tmpfs",
        MsFlags::MS_NOSUID
            .union(MsFlags::MS_NODEV)
            .union(MsFlags::MS_STRICTATIME),
        Some("mode=01777"),
    ),
    (
        "devpts",
        "/dev/pts",
        "devpts",
        MsFlags::MS_NOSUID.union(MsFlags::MS_NOEXEC),
        Some("mode=0620,gid=5"),
    ),
    (
        "tmpfs",
        "/run",
        "tmpfs",
        MsFlags::MS_NOSUID
            .union(MsFlags::MS_NODEV)
            .union(MsFlags::MS_STRICTATIME),
        Some("size=20%,nr_inodes=800k"),
    ),
];

/// Mount default filesystems from [`DEFAULT_MOUNT_TABLE`].
pub fn default_mounts() -> Result<(), Box<dyn std::error::Error>> {
    for entry in DEFAULT_MOUNT_TABLE {
        Mount::from(entry).mount()?;
    }
    Ok(())
}
