//! MIA installation library.
//!
//! Installs MIA, its dependencies and configuration files.
//!
//! This library is self-contained: all required binaries are baked inside.

use anyhow::{bail, Context, Result};
use log::{debug, info};
use mia_rt_config::MiaRuntimeConfig;
use std::fs;
use std::io::{BufRead, Write};
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
    run_command(&["mkdir", "-p", full_mia_path.to_str().unwrap()], true)?;

    install_mia_binary(&full_mia_path).context("install mia binary")?;

    if !config.no_symlink {
        install_mia_symlink(config).context("install mia symlink")?;
    }

    install_kmod(&full_mia_path, &config.install_path).context("install kmod")?;

    generate_rt_config(&full_mia_path, config).context("generate runtime config")?;

    run_command(&["mkdir", "-p", config.prefix.join("proc").to_str().unwrap()], true)?;

    info!("MIA installation completed");

    let installed_size =
        fs_extra::dir::get_size(&full_mia_path).context("calculate installed size")?;

    info!("installed size: {}KB", installed_size / 1024);

    Ok(())
}

fn install_mia_binary(path: &Path) -> Result<()> {
    let mia_bin_path = path.join(MIA_BIN_NAME);
    debug!("writing file {}", mia_bin_path.display());

    let mut child = std::process::Command::new("sudo")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .args(["tee", mia_bin_path.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("spawn tee command for mia binary")?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(MIA_BIN)
        .context("write mia file")?;
    child.wait().context("wait for tee command")?;
    run_command(&["chmod", "755", mia_bin_path.to_str().unwrap()], true)
        .context("change mod for mia binary")?;
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
        run_command(&["mkdir", "-p", symlink_dir.to_str().unwrap()], true)
            .context("create mia symlink directory")?;
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
            run_command(&["rm", full_symlink_path.to_str().unwrap()], true)
                .context("remove symlink")?;
        } else {
            bail!("symlink {} already exists", config.symlink_path.display());
        }
    }

    debug!(
        "creating symlink {} -> {}",
        full_symlink_path.display(),
        mia_bin_path.display()
    );
    run_command(
        &[
            "ln",
            "-s",
            mia_bin_path.to_str().unwrap(),
            full_symlink_path.to_str().unwrap(),
        ],
        true,
    )
    .context("create mia symlink")?;

    Ok(())
}

fn install_kmod(full_path: &Path, install_path: &Path) -> Result<()> {
    info!("installing kmod");
    let kmod_bin_path = full_path.join(KMOD_BIN_NAME);
    debug!("writing file {}", kmod_bin_path.display());

    let mut child = std::process::Command::new("sudo")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .args(["tee", kmod_bin_path.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("spawn tee command for kmod binary")?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(KMOD_BIN)
        .context("write kmod file")?;
    child.wait().context("wait for tee command")?;
    run_command(&["chmod", "755", kmod_bin_path.to_str().unwrap()], true)
        .context("change mod for kmod binary")?;

    let symlinks = ["depmod", "insmod", "lsmod", "modinfo", "modprobe", "rmmod"];
    let kmod_target_path = install_path.join(KMOD_BIN_NAME);
    for symlink in symlinks {
        let symlink_path = full_path.join(symlink);
        debug!(
            "creating symlink {} -> {}",
            symlink_path.display(),
            kmod_target_path.display()
        );
        run_command(
            &[
                "ln",
                "-s",
                kmod_target_path.to_str().unwrap(),
                symlink_path.to_str().unwrap(),
            ],
            true,
        )
        .context(format!("create {} symlink", symlink_path.display()))?;
    }

    Ok(())
}

fn generate_rt_config(full_path: &Path, install_config: &InstallConfig) -> Result<()> {
    info!("generating MIA runtime config");

    if install_config.rt_config_file.is_none() && install_config.rt_config.is_none() {
        bail!("no runtime config provided");
    }

    if install_config.rt_config_file.is_some() && install_config.rt_config.is_some() {
        bail!("two runtime configs provided at the same time");
    }

    let rt_config = if let Some(rt_config) = &install_config.rt_config {
        rt_config.clone()
    } else {
        // Safety: safe to unwrap because of the checks above.
        let rt_config_file = install_config.rt_config_file.clone().unwrap();
        debug!("reading config file {}", rt_config_file.display());
        let rt_config_file = fs::File::open(rt_config_file).context("open runtime config file")?;
        // NOTE: we deserialize config to ensure it's well-formed
        serde_yaml::from_reader(rt_config_file).context("deserialize runtime config")?
    };
    debug!("rt_config={:?}", &rt_config);

    let rt_config_string = serde_yaml::to_string(&rt_config).context("serialize runtime config")?;
    info!("generated config:");
    for line in rt_config_string.lines() {
        info!("  {}", line);
    }

    let rt_config_path = full_path.join(RT_CONFIG_FILENAME);

    let mut child = std::process::Command::new("sudo")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .args(["tee", rt_config_path.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("spawn tee command for runtime config file")?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(rt_config_string.as_bytes())
        .context("write runtime config file")?;
    child.wait().context("wait for tee command")?;

    Ok(())
}

fn run_command(commands: &[&str], as_root: bool) -> Result<()> {
    let program = if as_root { "sudo" } else { commands[0] };
    let args = if as_root { commands } else { &commands[1..] };

    debug!("running command: {program} {:?}", args);

    let mut child = std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Could not capture stdout."))?;

    let reader = std::io::BufReader::new(stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| debug!(target: commands[0], "{}", line));

    let output = child
        .wait_with_output()
        .context("Failed to wait for command")?;
    if output.status.success() {
        Ok(())
    } else {
        String::from_utf8(output.stderr)
            .context("Failed to parse command stderr")?
            .lines()
            .for_each(|line| debug!(target: commands[0], "{}", line));
        Err(anyhow::anyhow!(
            "Command failed with status {}",
            output.status
        ))
    }
}

// TODO(aleasims): replace ugly commands with pure Rust code when able 
