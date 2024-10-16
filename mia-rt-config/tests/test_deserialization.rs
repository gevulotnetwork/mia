use mia_rt_config::MiaRuntimeConfig;
use serde_yaml;

#[test]
fn test_deserialization_version_ok() -> anyhow::Result<()> {
    let result = serde_yaml::from_str::<MiaRuntimeConfig>("version: 1");
    assert_eq!(result.unwrap().version, 1);
    Ok(())
}

#[test]
fn test_deserialization_version_unsupported() -> anyhow::Result<()> {
    let result = serde_yaml::from_str::<MiaRuntimeConfig>("version: 2");
    assert!(result.is_err());
    Ok(())
}

const DEFAULT_MIA_CONFIG: &str = "
version: 1
mounts:
  - source: proc
    target: /proc
    fstype: proc
  - source: 0
    target: /mnt/mia-rt-config
kernel-modules:
  - nvidia
  - nvidia_uvm
bootcmd:
  - echo \"running MIA config\"
";

#[test]
fn test_deserialization_default_mia_config() -> anyhow::Result<()> {
    let _result = serde_yaml::from_str::<MiaRuntimeConfig>(DEFAULT_MIA_CONFIG)?;
    Ok(())
}
