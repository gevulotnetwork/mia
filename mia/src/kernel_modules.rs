use std::path::PathBuf;
use std::process;

pub struct Modules {
    modules: Vec<String>,
    modprobe_path: PathBuf,
}

impl TryFrom<Vec<&String>> for Modules {
    type Error = Box<dyn std::error::Error>;

    fn try_from(modules: Vec<&String>) -> Result<Self, Self::Error> {
        let modprobe_path = PathBuf::from(Self::DEFAULT_MODPROBE_PATH);
        if !modprobe_path.exists() {
            return Err(Box::from("modprobe not found"));
        }
        println!("[MIA] module: using {}", modprobe_path.display());
        let modules = modules.into_iter().cloned().collect();
        Ok(Self {
            modules,
            modprobe_path,
        })
    }
}

impl Modules {
    const DEFAULT_MODPROBE_PATH: &'static str = "/usr/lib/mia/modprobe";

    pub fn load(&self) -> Result<(), Box<dyn std::error::Error>> {
        for module in &self.modules {
            println!("[MIA] module: {}", module);
            let mut child = process::Command::new(self.modprobe_path.as_os_str())
                .arg(module)
                .spawn()?;
            child.wait()?;
        }
        Ok(())
    }
}
