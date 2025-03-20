use nix::errno::Errno;
use once_cell::sync::OnceCell;
use qemu_exit::{QEMUExit, X86};

const TARGET: &str = "qemu-debug-exit";

/// QEMU exit handler.
pub static QEMU_EXIT_HANDLER: OnceCell<X86> = OnceCell::new();

/// Setup QEMU exit handler and grant the process permissions to write to debug port `iobase`.
pub fn setup(
    iobase: u16,
    iosize: u64,
    success_code: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!(target: TARGET, "grant I/O permissions for port 0x{:x}", iobase);
    let ret = unsafe { libc::ioperm(iobase.into(), iosize, true.into()) };
    if ret != 0 {
        return Err(Box::from(format!(
            "I/O permission for port 0x{:x} failed: {} (code {})",
            iobase,
            Errno::from_raw(ret).desc(),
            ret
        )));
    }
    // Init handler at the end so it won't be available if port permissions failed.
    log::info!(
        target: TARGET,
        "setup QEMU exit handler (success code 0x{:x})",
        success_code
    );
    let _ = QEMU_EXIT_HANDLER.get_or_init(|| X86::new(iobase, success_code));
    Ok(())
}

fn exit_error() {
    log::info!(target: TARGET, "exiting QEMU with error");
    if let Some(handler) = QEMU_EXIT_HANDLER.get() {
        crate::pre_exit::flush();
        handler.exit_failure()
    }
    log::error!(target: TARGET, "QEMU exit handler is not set");
}

fn exit_success() {
    log::info!(target: TARGET, "exiting QEMU with success");
    if let Some(handler) = QEMU_EXIT_HANDLER.get() {
        crate::pre_exit::flush();
        handler.exit_success()
    }
    log::error!(target: TARGET, "QEMU exit handler is not set");
}

/// Try exiting QEMU with debug code.
/// `error` specifies whether exit with error or with success.
/// This functions returns only if exiting QEMU failed (exit handler is not set).
pub fn exit(error: bool) {
    if error {
        exit_error()
    } else {
        exit_success()
    }
}
