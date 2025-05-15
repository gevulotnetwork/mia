use gevulot_rs::runtime_config::{self, DebugExit, RuntimeConfig};

use crate::command::Command;
use crate::modprobe::Modprobe;
use crate::mount::Mount;
use crate::qemu;

const TARGET: &str = "rt-config";

pub fn load(mut path: String) -> Result<Command, Box<dyn std::error::Error>> {
    let mut cmd: Option<Command> = None;
    let modprobe = Modprobe::init()?;

    log::info!(target: TARGET, "version {}", runtime_config::VERSION);

    let mut bunding = std::process::Command::new("ls");

    let mut debug_cmd = bunding.arg("-l").arg("/mnt/gevulot");
    let out = debug_cmd.output().unwrap();
    log::info!("result: {:?}", String::from_utf8(out.stdout));
    log::info!("result err: {:?}", String::from_utf8(out.stderr));

    let mut bunding = std::process::Command::new("ls");

    let mut debug_cmd = bunding.arg("-l").arg("/mnt/gevulot/rt-config");
    let out = debug_cmd.output().unwrap();
    log::info!("result: {:?}", String::from_utf8(out.stdout));
    log::info!("result err: {:?}", String::from_utf8(out.stderr));

    let mut bunding = std::process::Command::new("cat");

    let mut debug_cmd = bunding.arg("/mnt/gevulot/rt-config/config.yaml");
    let out = debug_cmd.output().unwrap();
    log::info!("result: {:?}", String::from_utf8(out.stdout));
    log::info!("result err: {:?}", String::from_utf8(out.stderr));

    loop {
        log::info!(target: TARGET, "loading {}", &path);

        let config_file = std::fs::File::open(&path)?;
        log::info!(target: TARGET, "opening {}", &path);
        let config: RuntimeConfig = serde_yaml::from_reader(config_file)?;
        log::info!(target: TARGET, "config {:?}", &config);

        if let Some(DebugExit::X86 {
            iobase,
            iosize,
            success_code,
        }) = &config.debug_exit
        {
            qemu::setup(*iobase, *iosize as u64, *success_code)?;
        }

        for mount in &config.mounts {
            Mount::try_from(mount)?.mount()?;
        }

        for env in &config.env {
            std::env::set_var(env.key.clone(), env.value.clone());
            log::info!(target: TARGET, "env set: {}={}", &env.key, &env.value);
        }

        if let Some(working_dir) = &config.working_dir {
            std::env::set_current_dir(working_dir)?;
            log::info!(target: TARGET, "working dir set: {}", working_dir);
        }

        for module in &config.kernel_modules {
            modprobe.load(module)?;
        }

        for cmd in &config.bootcmd {
            log::info!(target: TARGET, "bootcmd: {}", cmd.join(" "));
            if cmd.is_empty() {
                return Err(Box::from("no command to run found"));
            }
            Command::new(cmd[0].clone(), cmd[1..].to_vec()).run()?;
        }

        if let Some(command) = &config.command {
            cmd = Some(Command::new(command.clone(), config.args.clone()));
        }

        if let Some(follow_config) = &config.follow_config {
            path = follow_config.clone();
        } else {
            break;
        }
    }

    if cmd.is_none() {
        log::error!(target: TARGET, "no command to run found");
        return Err(Box::from("no command to run found"));
    }

    Ok(cmd.unwrap())
}
