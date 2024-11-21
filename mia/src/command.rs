use std::process;

const TARGET: &str = "command";

pub struct Command {
    command: String,
    args: Vec<String>,
}

impl Command {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut command = process::Command::new(self.command.as_str());
        log::info!(
            target: TARGET,
            "{}",
            std::iter::once(&self.command)
                .chain(self.args.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        );
        for arg in &self.args {
            command.arg(arg);
        }
        let mut child = command.spawn()?;
        let status = child.wait()?; // Reap the child process to avoid zombie processes
        if !status.success() {
            return Err(Box::from(format!(
                "command `{}` failed with code: {}",
                &self.command,
                status
                    .code()
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or("<unknown>".to_string())
            )));
        }
        Ok(())
    }
}
