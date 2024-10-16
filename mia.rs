use nix::mount::{mount, MsFlags};
use std::fmt::Debug;
use std::path::PathBuf;
use std::process;

#[derive(Debug)]
struct Mount {
    source: String,
    target: String,
    fstype: String,
    data: String,
}

impl Mount {
    fn new(source: String, target: String, fstype: String, data: String) -> Self {
        Self {
            source,
            target,
            fstype,
            data,
        }
    }

    fn mount(&self) -> Result<(), Box<dyn std::error::Error>> {
        mount(
            Some(self.source.as_str()),
            self.target.as_str(),
            Some(self.fstype.as_str()),
            MsFlags::empty(),
            Some(self.data.as_str()),
        )?;
        Ok(())
    }
}

#[derive(Debug)]
struct Mounts(Vec<Mount>);

impl From<Vec<&String>> for Mounts {
    fn from(mounts: Vec<&String>) -> Self {
        Self(
            mounts
                .iter()
                .map(|m| {
                    let parts: Vec<&str> = m.split(':').collect();
                    let source = parts.first().unwrap().to_string();
                    let target = parts.get(1).unwrap_or(&"").to_string();
                    let fstype = parts.get(2).unwrap_or(&"9p").to_string();
                    let data = parts
                        .get(3)
                        .unwrap_or(&"trans=virtio,version=9p2000.L")
                        .to_string();
                    Mount::new(source, target, fstype, data)
                })
                .collect(),
        )
    }
}

impl Mounts {
    fn mount(&self) -> Result<(), Box<dyn std::error::Error>> {
        for mount in &self.0 {
            println!(
                "[MIA] mount: {}:{}:{}:{}",
                &mount.source, &mount.target, &mount.fstype, &mount.data
            );
            mount.mount()?;
        }
        Ok(())
    }
}

struct Modules {
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

    fn load(&self) -> Result<(), Box<dyn std::error::Error>> {
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

struct Command {
    env: Vec<(String, String)>,
    working_dir: Option<String>,
    command: String,
    args: Vec<String>,
}

impl Command {
    fn new(command: &str, args: Vec<&str>, env: Vec<&str>, working_dir: Option<&str>) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|a| a.to_string()).collect(),
            working_dir: working_dir.map(ToString::to_string),
            env: env
                .into_iter()
                .filter_map(|vardef| vardef.split_once("="))
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut command = process::Command::new(self.command.as_str());
        if let Some(working_dir) = &self.working_dir {
            println!("[MIA] working dir set: {}", working_dir);
            command.current_dir(working_dir);
        }
        print!("[MIA] env: ");
        for (var, value) in &self.env {
            print!("{}={} ", var, value);
            command.env(var, value);
        }
        println!();
        print!("[MIA] run: {} ", &self.command);
        for arg in &self.args {
            print!("{} ", arg);
            command.arg(arg);
        }
        println!();
        let mut child = command.spawn()?;
        child.wait()?; // Reap the child process to avoid zombie processes
        Ok(())
    }
}

fn shutdown() -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::reboot::{reboot, RebootMode};

    reboot(RebootMode::RB_POWER_OFF)?;
    Ok(())
}

fn app() -> Result<(), Box<dyn std::error::Error>> {
    println!("[MIA] start");

    let args: Vec<String> = std::env::args().collect();
    let mut modules = Vec::new();
    let mut mounts = Vec::new();
    let mut env_vars = Vec::new();
    let mut working_dir = None;
    let mut cmd_args = Vec::new();
    let mut is_mount = false;
    let mut is_module = false;
    let mut is_env = false;
    let mut is_wd = false;

    for arg in &args[1..] {
        match arg.as_str() {
            // arg is flag
            "--mount" => {
                is_mount = true;
            }
            "--module" => {
                is_module = true;
            }
            "--env" => {
                is_env = true;
            }
            "--wd" => {
                is_wd = true;
            }
            // arg is flag value
            _ if is_mount => {
                mounts.push(arg);
                is_mount = false;
            }
            _ if is_module => {
                modules.push(arg);
                is_module = false;
            }
            value if is_env => {
                env_vars.push(value);
                is_env = false;
            }
            value if is_wd => {
                working_dir = Some(value);
                is_wd = false;
            }
            // arg is final command part
            value => {
                cmd_args.push(value);
            }
        }
    }

    let command = cmd_args.first().ok_or("Command is required")?;

    if !modules.is_empty() {
        let modules = Modules::try_from(modules)?;
        modules.load()?;
    }

    let mounts = Mounts::from(mounts);
    mounts.mount()?;

    let cmd = Command::new(command, cmd_args[1..].to_vec(), env_vars, working_dir);

    cmd.run()?;

    shutdown()?;

    Ok(())
}

fn main() {
    if let Err(e) = app() {
        eprintln!("Error: {}", e);
    }
    if let Err(e) = shutdown() {
        eprintln!("Shutdown Error: {}", e);
    }
}
