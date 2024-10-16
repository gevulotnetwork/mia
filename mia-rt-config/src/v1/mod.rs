use serde::de::{Deserializer, Error};
use serde::{Deserialize, Serialize};

/// Current config version.
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
    pub flags: Option<u32>,
    pub data: Option<String>,
}

/// MIA runtime config.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

    /// Default mounts (/proc, /tmp etc.).
    #[serde(default)]
    pub default_mounts: bool,

    /// Kernel modules.
    #[serde(default)]
    pub kernel_modules: Vec<String>,

    /// Boot commands.
    #[serde(default)]
    pub bootcmd: Vec<String>,
}

impl MiaRuntimeConfig {
    /// Update current config with values from other one.
    ///
    /// Different fields of the config are updated differently:
    ///  - `version` and `default_mounts` will be overwritten
    ///  - `command` and `working_dir` will be overwritten if they are `Some(_)`
    ///  - `args` will be overwritten if `command` was overwritten
    ///  - `env`, `mounts`, `kernel_modules` and `bootcmd` will be appended
    pub fn update(&mut self, other: &MiaRuntimeConfig) {
        self.version = other.version;
        self.default_mounts = other.default_mounts;
        if other.command.is_some() {
            self.command = other.command.clone();
            self.args = other.args.clone();
        }
        if other.working_dir.is_some() {
            self.working_dir = other.working_dir.clone();
        }
        self.env.append(&mut other.env.clone());
        self.mounts.append(&mut other.mounts.clone());
        self.kernel_modules.append(&mut other.kernel_modules.clone());
    }
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
