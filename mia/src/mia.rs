use nix::sys::reboot::RebootMode;

mod command;
mod logger;
mod modprobe;
mod mount;
mod pre_exit;
mod qemu;
mod rt_config;

const TARGET: &str = "";

fn shutdown() -> ! {
    log::info!(target: TARGET, "shutdown");
    let Err(err) = nix::sys::reboot::reboot(RebootMode::RB_POWER_OFF);
    log::error!(target: TARGET, "shutdown error: {} ({})", err.desc(), err as i32);
    // None of the errors from libc::reboot can happened here, because this process must always have
    // right permissions. So this code can be marked as unreachable.
    // If an error is encountered, then it's either an error in `nix` or inappropriate use of MIA.
    // In both of that cases panicing here and causing kernel panic is okay.
    unreachable!("internal error on poweroff")
}

const MIA_CONFIG_PATH: &str = "/usr/lib/mia/config.yaml";

fn start() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup();
    log::info!(target: TARGET, "start");

    let cmd = rt_config::load(MIA_CONFIG_PATH.to_string())?;

    log::info!(target: TARGET, "run main process");
    cmd.run()?;

    Ok(())
}

// Init process should never return.
fn main() -> ! {
    let err = if let Err(e) = start() {
        log::error!(target: TARGET, "{}", e);
        true
    } else {
        false
    };

    // Sync filesystems before attempting to shutdown.
    nix::unistd::sync();

    if qemu::QEMU_EXIT_HANDLER.get().is_some() {
        qemu::exit(err);
    }
    // If no exit handler was set, perform simple shutdown.
    pre_exit::flush();
    shutdown()
}
