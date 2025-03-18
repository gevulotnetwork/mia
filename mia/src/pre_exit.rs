use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// Flush stdout and stderr.
pub fn flush() {
    const FLUSHING_DELAY: Duration = Duration::from_secs(1);

    if let Err(err) = io::stdout().flush() {
        log::error!("flushing stdout: {}", err);
        let _ = io::stdout().flush();
    }

    if let Err(err) = io::stderr().flush() {
        log::error!("flushing stderr: {}", err);
        let _ = io::stdout().flush();
    }

    // Give some time to flush stdout/stderr
    thread::sleep(FLUSHING_DELAY);
}
