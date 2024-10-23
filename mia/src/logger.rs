use env_logger::{Builder, Env, Target};
use std::io::Write;

pub fn setup() {
    let logger_env = Env::default().filter_or("MIA_LOG", "info");
    Builder::from_env(logger_env)
        .target(Target::Stdout)
        .format(|buf, record| {
            if record.target().is_empty() {
                writeln!(buf, "[MIA] [{}] {}", record.level(), record.args())
            } else {
                writeln!(
                    buf,
                    "[MIA] [{}] {}: {}",
                    record.level(),
                    record.target(),
                    record.args()
                )
            }
        })
        .init();
}
