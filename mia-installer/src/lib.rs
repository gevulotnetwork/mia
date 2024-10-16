//! MIA installation library.
//!
//! Installs MIA, its dependencies and configuration files.
//!
//! This library is self-contained: all required binaries are baked inside.

use anyhow::{bail, Context, Result};
use log::{debug, info};
use mia_rt_config::MiaRuntimeConfig;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{self, Path, PathBuf};
use structopt::StructOpt;

/// `mia` binary.
// TODO: waiting for bindeps to be stabilized
const MIA_BIN: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/x86_64-unknown-linux-gnu/release/mia"
));

/// Name of the file to create.
const MIA_BIN_NAME: &str = "mia";

/// `kmod` binary.
const KMOD_BIN: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pre-built-deps/kmod"));

/// Name of the file to create.
const KMOD_BIN_NAME: &str = "kmod";

const DEFAULT_INSTALL_PREFIX: &str = "";

const DEFAULT_INSTALL_PATH: &str = "/usr/lib/mia";

const DEFAULT_SYMLINK_PATH: &str = "/sbin/init";

const RT_CONFIG_FILENAME: &str = "config.yaml";

#[derive(Clone, Debug, StructOpt)]
#[structopt(about = "Installs MIA, its dependencies and configuration files.")]
pub struct InstallConfig {
    /// Installation prefix.
    #[structopt(long, required = false, default_value = DEFAULT_INSTALL_PREFIX)]
    pub prefix: PathBuf,

    /// Installation path. This path is going to be joined with `prefix`.
    #[structopt(long, default_value = DEFAULT_INSTALL_PATH)]
    pub install_path: PathBuf,

    /// Don't mount default mounts (`/proc`, `/tmp` etc.).
    #[structopt(long)]
    pub no_default_mounts: bool,

    /// Module to load at boot time. May be passed multiple times.
    #[structopt(long = "kernel-module", name = "kernel-module")]
    pub kernel_modules: Vec<String>,

    /// Don't create symlink to mia.
    #[structopt(long)]
    pub no_symlink: bool,

    /// MIA symlink path. If `no_symlink` is set, this will be ignored.
    #[structopt(long = "symlink-path", default_value = DEFAULT_SYMLINK_PATH)]
    pub symlink_path: PathBuf,

    /// Overwrite symlink if it already exists.
    #[structopt(long)]
    pub overwrite_symlink: bool,

    /// Additional MIA runtime config.
    ///
    /// This works the same way as `rt_config_file`.
    /// If `rt_config_file` is provided, this config will be ignored.
    #[structopt(skip)]
    pub rt_config: Option<MiaRuntimeConfig>,

    /// Read additional MIA runtime config from file.
    ///
    /// This config is going to be merged with default generated one.
    /// Any conflicting options in it will be updated.
    ///
    /// See [`MiaRuntimeConfig::update`].
    #[structopt(long = "runtime-config", name = "runtime-config")]
    pub rt_config_file: Option<PathBuf>,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            prefix: PathBuf::from(DEFAULT_INSTALL_PREFIX),
            install_path: PathBuf::from(DEFAULT_INSTALL_PATH),
            no_default_mounts: false,
            kernel_modules: Vec::new(),
            no_symlink: false,
            symlink_path: PathBuf::from(DEFAULT_SYMLINK_PATH),
            overwrite_symlink: false,
            rt_config: None,
            rt_config_file: None,
        }
    }
}

/// Install MIA with given installation config.
pub fn install(config: &InstallConfig) -> Result<()> {
    let full_mia_path = config.prefix.join(
        config
            .install_path
            .as_path()
            .strip_prefix(path::MAIN_SEPARATOR_STR)
            .context("strip path separator from install path")?,
    );
    info!("installing MIA to {}", full_mia_path.display());

    debug!("creating directory {}", full_mia_path.display());
    fs::create_dir_all(&full_mia_path).context("create MIA directory")?;

    install_mia_binary(&full_mia_path).context("install mia binary")?;

    if !config.no_symlink {
        install_mia_symlink(config).context("install mia symlink")?;
    }

    install_kmod(&full_mia_path, &config.install_path).context("install kmod")?;

    generate_rt_config(&full_mia_path, config).context("generate runtime config")?;

    info!("MIA installation completed");

    let installed_size =
        fs_extra::dir::get_size(&full_mia_path).context("calculate installed size")?;

    info!("installed size: {}KB", installed_size / 1024);

    Ok(())
}

fn install_mia_binary(path: &Path) -> Result<()> {
    let mia_bin_path = path.join(MIA_BIN_NAME);
    debug!("writing file {}", mia_bin_path.display());
    let mut mia_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o755)
        .open(&mia_bin_path)
        .context("create mia file")?;
    mia_file.write(MIA_BIN).context("write mia file")?;
    Ok(())
}

fn install_mia_symlink(config: &InstallConfig) -> Result<()> {
    debug_assert!(!config.no_symlink);
    info!("symlinking MIA");
    let mia_bin_path = config.install_path.join(MIA_BIN_NAME);

    if let Some(parent_dir) = config.symlink_path.parent() {
        let symlink_dir = config.prefix.join(
            parent_dir
                .strip_prefix(path::MAIN_SEPARATOR_STR)
                .context("strip path separator from symlink base path")?,
        );
        debug!("ensure directory {} exists", symlink_dir.display());
        fs::create_dir_all(&symlink_dir).context("create mia symlink directory")?;
    }

    let full_symlink_path = config.prefix.join(
        config
            .symlink_path
            .as_path()
            .strip_prefix(path::MAIN_SEPARATOR_STR)
            .context("strip path separator from symlink path")?,
    );

    if full_symlink_path.exists() {
        if config.overwrite_symlink {
            debug!(
                "symlink {} alreasy exists, removing it",
                config.symlink_path.display()
            );
            fs::remove_file(&full_symlink_path).context("remove symlink")?;
        } else {
            bail!("symlink {} already exists", config.symlink_path.display());
        }
    }

    debug!(
        "creating symlink {} -> {}",
        full_symlink_path.display(),
        mia_bin_path.display()
    );
    unix::fs::symlink(mia_bin_path, &full_symlink_path).context("create mia symlink")?;

    Ok(())
}

fn install_kmod(full_path: &Path, install_path: &Path) -> Result<()> {
    info!("installing kmod");
    let kmod_bin_path = full_path.join(KMOD_BIN_NAME);
    debug!("writing file {}", kmod_bin_path.display());
    let mut kmod_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o755)
        .open(&kmod_bin_path)
        .context("create kmod file")?;
    kmod_file.write(KMOD_BIN).context("write kmod file")?;

    let symlinks = ["depmod", "insmod", "lsmod", "modinfo", "modprobe", "rmmod"];
    let kmod_target_path = install_path.join(KMOD_BIN_NAME);
    for symlink in symlinks {
        let symlink_path = full_path.join(symlink);
        debug!(
            "creating symlink {} -> {}",
            symlink_path.display(),
            kmod_target_path.display()
        );
        unix::fs::symlink(&kmod_target_path, &symlink_path)
            .context(format!("create {} symlink", symlink_path.display()))?;
    }

    Ok(())
}

fn generate_rt_config(full_path: &Path, config: &InstallConfig) -> Result<()> {
    info!("generating MIA runtime config");

    let rt_config_path = full_path.join(RT_CONFIG_FILENAME);

    let mut mounts = Vec::new();
    if !config.no_default_mounts {
        // TODO: maybe we need other default mounts
        // TODO: ensure target dir exists
        mounts.push(mia_rt_config::Mount {
            source: "proc".into(),
            target: "/proc".into(),
            fstype: Some("proc".into()),
            flags: None,
            data: None,
        });
    }

    let mut rt_config = MiaRuntimeConfig {
        version: mia_rt_config::VERSION,
        command: None,
        args: Vec::new(),
        env: Vec::new(),
        working_dir: None,
        mounts,
        default_mounts: true,
        kernel_modules: config.kernel_modules.clone(),
        bootcmd: Vec::new(),
    };

    let mut update_config = config.rt_config.clone();
    if let Some(rt_config_file) = &config.rt_config_file {
        debug!("reading config file {}", rt_config_file.display());
        let rt_config_file = fs::File::open(rt_config_file).context("open runtime config file")?;
        update_config = Some(
            serde_yaml::from_reader(rt_config_file).context("deserialize update runtime config")?,
        );
    }

    if let Some(update_config) = update_config {
        debug!("updating default config with: {:?}", &update_config);
        rt_config.update(&update_config);
    }

    let rt_config_string = serde_yaml::to_string(&rt_config).context("serialize runtime config")?;
    info!("generated config:");
    for line in rt_config_string.lines() {
        info!("  {}", line);
    }

    debug!("writing config to {}", rt_config_path.display());
    let mut rt_config_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o664)
        .open(&rt_config_path)
        .context("create runtime config file")?;
    rt_config_file
        .write(rt_config_string.as_bytes())
        .context("write runtime config file")?;

    Ok(())
}
