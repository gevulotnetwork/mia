//! MIA installation library.
//!
//! Installs MIA, its dependencies and configuration files.
//!
//! This library is self-contained: all required binaries are baked inside.

use anyhow::{anyhow, bail, Context, Result};
use flate2::read::GzDecoder;
use log::{debug, info};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{self, Path, PathBuf};
use std::process::{Command, Stdio};
use structopt::StructOpt;
use tempdir::TempDir;
use tokio::runtime;
use url::Url;

// Re-export runtime-config to avoid interdependencies between `gevulot-rs`,
// `mia` and `gvltctl`.
pub use gevulot_rs::runtime_config::{self, RuntimeConfig};

/// Owner of MIA repository on GitHub.
const GITHUB_MIA_OWNER: &str = "gevulotnetwork";

/// MIA repository name on GitHub.
const GITHUB_MIA_REPO: &str = "mia";

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
    /// MIA version to install.
    ///
    /// Examples: `0.1.0`, `latest`, `file:/path/to/mia/binary`.
    ///
    /// MIA will be loaded from GitHub Releases of its repository.
    #[structopt(long, required = false, default_value = "latest")]
    pub mia_version: String,

    /// MIA platform to install.
    ///
    /// If `--mia-version file:PATH` is used, this option is ignored.
    ///
    /// Example: `x86_64-unknown-linux-gnu`.
    #[structopt(long = "platform", required = true)]
    pub mia_platform: String,

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

    /// MIA runtime config.
    #[structopt(skip)]
    pub rt_config: Option<RuntimeConfig>,

    /// Read additional MIA runtime config from file.
    #[structopt(long = "runtime-config", name = "runtime-config")]
    pub rt_config_file: Option<PathBuf>,

    /// Run installation commands as root.
    #[structopt(long)]
    pub as_root: bool,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            mia_version: "latest".to_string(),
            mia_platform: "unknown".to_string(),
            prefix: PathBuf::from(DEFAULT_INSTALL_PREFIX),
            install_path: PathBuf::from(DEFAULT_INSTALL_PATH),
            no_symlink: false,
            symlink_path: PathBuf::from(DEFAULT_SYMLINK_PATH),
            overwrite_symlink: false,
            rt_config: None,
            rt_config_file: None,
            as_root: false,
        }
    }
}

/// Install MIA with given installation config.
pub fn install(config: &InstallConfig) -> Result<()> {
    let tmp = TempDir::new("mia-installer").context("create temp dir")?;
    debug!("using temp directory: {}", tmp.path().display());

    let mia_bin = get_mia(tmp.path(), &config.mia_version, &config.mia_platform)?;

    let full_mia_path = config.prefix.join(
        config
            .install_path
            .as_path()
            .strip_prefix(path::MAIN_SEPARATOR_STR)
            .context("strip path separator from install path")?,
    );
    info!("installing MIA to {}", full_mia_path.display());

    debug!("creating directory {}", full_mia_path.display());
    run_command(
        &["mkdir", "-p", full_mia_path.to_str().unwrap()],
        config.as_root,
    )?;

    install_mia_binary(&full_mia_path, &mia_bin, config.as_root).context("install mia binary")?;

    if !config.no_symlink {
        install_mia_symlink(config, config.as_root).context("install mia symlink")?;
    }

    install_kmod(&full_mia_path, &config.install_path, config.as_root).context("install kmod")?;

    generate_rt_config(&full_mia_path, config, config.as_root)
        .context("generate runtime config")?;

    run_command(
        &["mkdir", "-p", config.prefix.join("proc").to_str().unwrap()],
        config.as_root,
    )?;

    info!("MIA installation completed");

    let installed_size =
        fs_extra::dir::get_size(&full_mia_path).context("calculate installed size")?;

    info!("installed size: {}KB", installed_size / 1024);

    Ok(())
}

fn get_mia(tmp: &Path, version: &str, platform: &str) -> Result<PathBuf> {
    if let Some(version) = version.strip_prefix("file:") {
        info!("using MIA: {}", &version);
        Ok(PathBuf::from(version))
    } else {
        info!("downloading MIA: {} ({})", &version, &platform);
        let tmp = tmp.to_path_buf();
        let version = version.to_string();
        let platform = platform.to_string();
        if let Ok(handle) = runtime::Handle::try_current() {
            std::thread::spawn(move || handle.block_on(fetch_mia(tmp, version, platform)))
                .join()
                .map_err(|_| anyhow!("failed to join thread"))?
        } else {
            std::thread::spawn(|| {
                runtime::Runtime::new()?.block_on(fetch_mia(tmp, version, platform))
            })
            .join()
            .map_err(|_| anyhow!("failed to join thread"))?
        }
    }
}

async fn fetch_mia(tmp: PathBuf, version: String, platform: String) -> Result<PathBuf> {
    let octocrab = octocrab::instance();
    let repo = octocrab.repos(GITHUB_MIA_OWNER, GITHUB_MIA_REPO);
    let releases = repo.releases();

    let release = if version == "latest" {
        releases
            .get_latest()
            .await
            .context("get latest MIA release")?
    } else {
        // Expected format of MIA release tags is `mia-X.Y.Z`
        releases
            .get_by_tag(&format!("mia-{}", version))
            .await
            .context(format!("get MIA release {}", version))?
    };

    let version = release
        .tag_name
        .strip_prefix("mia-")
        .ok_or(anyhow!("invalid release tag"))?;
    debug!("resolved release version: mia-{}", &version);

    let asset_filename = format!("mia-{}-{}.tar.gz", &version, platform);
    debug!("searching for {} package", &asset_filename);

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_filename)
        .ok_or(anyhow!(
            "failed to find MIA package in release mia-{}",
            version
        ))?;

    let asset_dir = fetch_tar_gz(&tmp, &asset_filename, asset.browser_download_url.clone()).await?;

    Ok(asset_dir.join(MIA_BIN_NAME))
}

/// Fetch and unpack `.tar.gz` archive returning file to output directory.
async fn fetch_tar_gz(tmp: &Path, filename: &str, url: Url) -> Result<PathBuf> {
    debug!("fetching {}", &url);
    let response = reqwest::get(url).await.context("GET request")?;
    let bytes = response.bytes().await.context("read response body")?;
    let tar = GzDecoder::new(bytes.as_ref());
    let mut archive = tar::Archive::new(tar);
    let path = tmp.join(filename);
    debug!("unpacking archive to {}", path.display());
    archive.unpack(&path)?;
    Ok(path)
}

fn install_mia_binary(path: &Path, mia_source: &Path, as_root: bool) -> Result<()> {
    let mia_bin_path = path.join(MIA_BIN_NAME);
    debug!("copying to {}", mia_bin_path.display());
    run_command(
        &[
            "cp",
            mia_source.to_str().unwrap(),
            mia_bin_path.to_str().unwrap(),
        ],
        as_root,
    )
    .context("copy mia binary")?;
    run_command(&["chmod", "755", mia_bin_path.to_str().unwrap()], as_root)
        .context("change mod for mia binary")?;
    Ok(())
}

fn install_mia_symlink(config: &InstallConfig, as_root: bool) -> Result<()> {
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
        run_command(&["mkdir", "-p", symlink_dir.to_str().unwrap()], as_root)
            .context("create mia symlink directory")?;
    }

    let full_symlink_path = config.prefix.join(
        config
            .symlink_path
            .as_path()
            .strip_prefix(path::MAIN_SEPARATOR_STR)
            .context("strip path separator from symlink path")?,
    );

    if full_symlink_path.is_symlink() {
        if config.overwrite_symlink {
            debug!(
                "symlink {} alreasy exists, removing it",
                config.symlink_path.display()
            );
            run_command(&["rm", full_symlink_path.to_str().unwrap()], as_root)
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
        as_root,
    )
    .context("create mia symlink")?;

    Ok(())
}

fn install_kmod(full_path: &Path, install_path: &Path, as_root: bool) -> Result<()> {
    info!("installing kmod");
    let kmod_bin_path = full_path.join(KMOD_BIN_NAME);
    debug!("writing file {}", kmod_bin_path.display());

    let mut cmd = if as_root {
        Command::new("sudo")
    } else {
        Command::new("tee")
    };
    if as_root {
        cmd.args(["tee", kmod_bin_path.to_str().unwrap()]);
    } else {
        cmd.arg(kmod_bin_path.to_str().unwrap());
    }
    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn tee command for kmod binary")?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(KMOD_BIN)
        .context("write kmod file")?;
    child.wait().context("wait for tee command")?;
    run_command(&["chmod", "755", kmod_bin_path.to_str().unwrap()], as_root)
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
            as_root,
        )
        .context(format!("create {} symlink", symlink_path.display()))?;
    }

    Ok(())
}

fn generate_rt_config(
    full_path: &Path,
    install_config: &InstallConfig,
    as_root: bool,
) -> Result<()> {
    info!("generating MIA runtime config");

    if install_config.rt_config_file.is_some() && install_config.rt_config.is_some() {
        bail!("two runtime configs provided at the same time");
    }

    let rt_config = if let Some(rt_config) = &install_config.rt_config {
        rt_config.clone()
    } else if let Some(rt_config_file) = &install_config.rt_config_file {
        debug!("reading config file {}", rt_config_file.display());
        let rt_config_file = fs::File::open(rt_config_file).context("open runtime config file")?;
        // NOTE: we deserialize config to ensure it's well-formed
        serde_yaml::from_reader(rt_config_file).context("deserialize runtime config")?
    } else {
        debug!("creating default (empty) runtime config");
        RuntimeConfig {
            version: runtime_config::VERSION.to_string(),
            ..Default::default()
        }
    };
    debug!("rt_config={:?}", &rt_config);

    let rt_config_string = serde_yaml::to_string(&rt_config).context("serialize runtime config")?;
    info!("generated config:");
    for line in rt_config_string.lines() {
        info!("  {}", line);
    }

    let rt_config_path = full_path.join(RT_CONFIG_FILENAME);

    let mut cmd = if as_root {
        Command::new("sudo")
    } else {
        Command::new("tee")
    };
    if as_root {
        cmd.args(["tee", rt_config_path.to_str().unwrap()]);
    } else {
        cmd.arg(rt_config_path.to_str().unwrap());
    }

    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
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

    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Could not capture stdout."))?;

    let reader = std::io::BufReader::new(stdout);
    reader
        .lines()
        .map_while(Result::ok)
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
        Err(anyhow!("Command failed with status {}", output.status))
    }
}

// TODO(aleasims): replace ugly commands with pure Rust code when able
