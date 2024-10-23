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

const EXAMPLE_CONFIG: &str = "
version: 1
command: /prover
args: [--log, info]
env:
  - key: TMPDIR
    value: /tmp
working-dir: /
mounts:
  - source: input-1
    target: /input/1
default-mounts: true
kernel-modules:
  - nvidia
bootcmd:
  - [echo, booting]
follow-config: /mnt/gevulot-rt-config/config.yaml
";

#[test]
fn test_deserialization_example_config() -> anyhow::Result<()> {
    let result = serde_yaml::from_str::<MiaRuntimeConfig>(EXAMPLE_CONFIG)?;
    assert_eq!(result.command, Some("/prover".to_string()));
    assert_eq!(result.args, vec!["--log".to_string(), "info".to_string()]);
    assert_eq!(result.env.len(), 1);
    assert_eq!(result.env[0].key, "TMPDIR".to_string());
    assert_eq!(result.env[0].value, "/tmp".to_string());
    assert_eq!(result.working_dir, Some("/".to_string()));
    assert_eq!(result.mounts.len(), 1);
    assert_eq!(result.mounts[0].source, "input-1".to_string());
    assert_eq!(result.mounts[0].target, "/input/1".to_string());
    assert_eq!(result.mounts[0].fstype, None);
    assert_eq!(result.mounts[0].flags, None);
    assert_eq!(result.mounts[0].data, None);
    assert_eq!(result.default_mounts, true);
    assert_eq!(result.kernel_modules, vec!["nvidia".to_string()]);
    assert_eq!(result.bootcmd, vec![vec!["echo", "booting"]]);
    assert_eq!(result.follow_config, Some("/mnt/gevulot-rt-config/config.yaml".to_string()));
    Ok(())
}
