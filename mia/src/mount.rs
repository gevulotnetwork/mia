const TARGET: &str = "mount";

#[cfg(target_os = "linux")]
pub fn mount(
    source: Option<String>,
    target: String,
    fstype: Option<String>,
    flags: Option<u64>,
    data: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use nix::mount::MsFlags;

    let source = source.as_ref().map(String::as_str);
    let fstype = fstype.as_ref().map(String::as_str);
    let flags = if let Some(bits) = flags {
        MsFlags::from_bits(bits).ok_or("Invalid mount flags")?
    } else {
        MsFlags::empty()
    };
    let data = data.as_ref().map(String::as_str);
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

#[cfg(target_os = "macos")]
pub fn mount(
    source: Option<String>,
    target: String,
    flags: Option<i32>,
    data: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use nix::mount::MntFlags;

    let source = source.as_ref().map(String::as_str);
    let flags = if let Some(bits) = flags {
        MntFlags::from_bits(bits).ok_or("Invalid mount flags")?
    } else {
        MntFlags::empty()
    };

    let data = data.as_ref().map(String::as_str);
    log::info!(
        target: TARGET,
        "{}:{}:{}",
        source.unwrap_or(""),
        &target,
        data.unwrap_or("")
    );

    nix::mount::mount(source.unwrap(), target.as_str(), flags, data)?;
    Ok(())
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "macos")]
pub fn default_mounts() -> Result<(), Box<dyn std::error::Error>> {
    mount(Some("proc".to_string()), "/proc".to_string(), None, None)?;
    // TODO: which mounts do we need here?

    Ok(())
}
