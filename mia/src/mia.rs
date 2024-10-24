mod command;
mod logger;
mod modprobe;
mod mount;
mod rt_config;

const TARGET: &str = "";

#[cfg(target_os = "linux")]
fn shutdown() -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::reboot::RebootMode;
    nix::sys::reboot::reboot(RebootMode::RB_POWER_OFF)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn shutdown() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    Command::new("shutdown").arg("-h").arg("now").status()?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn shutdown() -> Result<(), Box<dyn std::error::Error>> {
    // Placeholder for other platforms
    println!("Shutdown not implemented for this platform");
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
