use nix::sys::reboot::RebootMode;

mod command;
mod logger;
mod modprobe;
mod mount;
mod rt_config;

const TARGET: &str = "";

fn shutdown() -> Result<(), Box<dyn std::error::Error>> {
    nix::sys::reboot::reboot(RebootMode::RB_POWER_OFF)?;
    Ok(())
}

const MIA_CONFIG_PATH: &str = "/usr/lib/mia/config.yaml";

fn start() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup();
    log::info!(target: TARGET, "start");

    let cmd = rt_config::load(MIA_CONFIG_PATH.to_string())?;

    log::info!(target: TARGET, "run main process");
    cmd.run()?;

    log::info!(target: TARGET, "shutdown");
    shutdown()?;

    Ok(())
}

fn main() {
    if let Err(e) = start() {
        log::error!(target: TARGET, "{}", e);
    }
    if let Err(e) = shutdown() {
        log::error!(target: TARGET, "Shutdown Error: {}", e);
    }
}
