use std::process;
use mia_rt_config as _;

mod mount;
mod kernel_modules;

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

struct CliArgs {
    pub mounts: Vec<String>,
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
        let modules = kernel_modules::Modules::try_from(modules)?;
        modules.load()?;
    }

    let mounts = mount::Mounts::from(mounts);
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
