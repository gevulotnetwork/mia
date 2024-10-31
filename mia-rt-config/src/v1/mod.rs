use serde::de::{Deserializer, Error};
use serde::{Deserialize, Serialize};

/// Config version.
pub const VERSION: u16 = 1;

/// Environment variable definition.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Env {
    pub key: String,
    pub value: String,
}

/// Mount definition.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Mount {
    pub source: String,
    pub target: String,
    pub fstype: Option<String>,
    pub flags: Option<u64>,
    pub data: Option<String>,
}

impl Mount {
    /// Create virtio 9p mount.
    pub fn virtio9p(source: String, target: String) -> Self {
        Self {
            source,
            target,
            fstype: Some("9p".to_string()),
            flags: None,
            data: Some("trans=virtio,version=9p2000.L".to_string()),
        }
    }
}

fn true_value() -> bool {
    true
}

/// MIA runtime config.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct MiaRuntimeConfig {
    /// Config version.
    #[serde(deserialize_with = "deserialize_version")]
    pub version: u16,

    /// Program to execute.
    pub command: Option<String>,

    /// Args to the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: Vec<Env>,

    /// Working directory.
    pub working_dir: Option<String>,

    /// Mounts.
    #[serde(default)]
    pub mounts: Vec<Mount>,

    /// Default mounts (/proc, /tmp etc.). Defaults to `true`.
    #[serde(default = "true_value")]
    pub default_mounts: bool,

    /// Kernel modules.
    #[serde(default)]
    pub kernel_modules: Vec<String>,

    /// Boot commands.
    #[serde(default)]
    pub bootcmd: Vec<Vec<String>>,

    /// Path to another runtime config file to apply after current one.
    ///
    /// This option allows to chain configs.
    /// Followed config will be accessed after all mounting done in the current.
    /// This means that new config may be located in mounted directory.
    pub follow_config: Option<String>,
}

/// Deserialize `u16` and compare it to `VERSION`.
fn deserialize_version<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let version = u16::deserialize(deserializer)?;
    if version != VERSION {
        return Err(D::Error::custom("MIA runtime config: unsupported version"));
    }
    Ok(version)
}

// TODO(aleasims): version must always be checked first
