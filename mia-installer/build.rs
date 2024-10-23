use std::env;
use std::process;

fn main() {
    // NOTE: this relies on the directory structures:
    // mia must be on the same level as mia-installer
    // TODO: wait for bindeps to be stabilized and re-write this.
    let mia_dir = format!("{}/../mia", env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = env::var("OUT_DIR").unwrap();

    eprintln!("mia_dir={}", &mia_dir);
    eprintln!("out_dir={}", &out_dir);

    println!("cargo::rerun-if-changed={}", mia_dir);

    // Filter all CARGO* env vars.
    // NOTE: This is required because MIA has its own Cargo config, which
    // will conflict with env and will not be used. For simplicity,
    // we just clear all Cargo-related variables to emulate the process
    // of normal MIA build like `cd mia && cargo b -r`
    let filtered_env = env::vars().filter(|&(ref var, _)| !var.starts_with("CARGO"));

    env::set_current_dir(&mia_dir).unwrap();
    // TODO: we could use --out-dir here to avoid relying on mias target
    // directory structure in the future, but its still unstable.
    let status = process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target-dir")
        .arg(&out_dir)
        .env_clear()
        .envs(filtered_env)
        .status()
        .unwrap();
    assert!(status.success());
}
