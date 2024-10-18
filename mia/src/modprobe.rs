use std::path::PathBuf;
use std::process;

const TARGET: &str = "modprobe";

const DEFAULT_MODPROBE_PATH: &str = "/usr/lib/mia/modprobe";

/// `modprobe` wrapper.
pub struct Modprobe {
    exec_path: PathBuf,
}

impl Modprobe {
    /// Initialize modprobe.
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let exec_path = PathBuf::from(DEFAULT_MODPROBE_PATH);
        if !exec_path.exists() {
            return Err(Box::from("modprobe not found"));
        }
        log::info!(target: TARGET, "using {}", exec_path.display());
        Ok(Self { exec_path })
    }

    /// Load kernel module.
    pub fn load(&self, module_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        log::info!(target: TARGET, "loading {}", module_name);
        let mut child = process::Command::new(self.exec_path.as_os_str())
            .arg(module_name)
            .spawn()?;
        child.wait()?;
        Ok(())
    }
}
