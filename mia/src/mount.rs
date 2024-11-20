use nix::mount::MsFlags;

const TARGET: &str = "mount";

pub fn mount(
    source: Option<String>,
    target: String,
    fstype: Option<String>,
    flags: Option<u64>,
    data: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = source.as_deref();
    let fstype = fstype.as_deref();
    let flags = if let Some(bits) = flags {
        MsFlags::from_bits(bits).ok_or("Invalid mount flags")?
    } else {
        MsFlags::empty()
    };
    let data = data.as_deref();
    log::info!(
        target: TARGET,
        "{}:{}:{}:{}",
        source.unwrap_or(""),
        &target,
        fstype.unwrap_or(""),
        data.unwrap_or("")
    );
    nix::mount::mount(source, target.as_str(), fstype, flags, data)?;
    Ok(())
}

pub fn default_mounts() -> Result<(), Box<dyn std::error::Error>> {
    mount(
        Some("proc".to_string()),
        "/proc".to_string(),
        Some("proc".to_string()),
        None,
        None,
    )?;
    // TODO: which mounts do we need here?

    Ok(())
}
