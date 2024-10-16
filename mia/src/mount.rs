use nix::mount::{mount, MsFlags};

#[derive(Debug)]
pub struct Mount {
    source: String,
    target: String,
    fstype: String,
    data: String,
}

impl Mount {
    pub fn new(source: String, target: String, fstype: String, data: String) -> Self {
        Self {
            source,
            target,
            fstype,
            data,
        }
    }

    pub fn mount(&self) -> Result<(), Box<dyn std::error::Error>> {
        mount(
            Some(self.source.as_str()),
            self.target.as_str(),
            Some(self.fstype.as_str()),
            MsFlags::empty(),
            Some(self.data.as_str()),
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Mounts(Vec<Mount>);

impl From<Vec<&String>> for Mounts {
    fn from(mounts: Vec<&String>) -> Self {
        Self(
            mounts
                .iter()
                .map(|m| {
                    let parts: Vec<&str> = m.split(':').collect();
                    let source = parts.first().unwrap().to_string();
                    let target = parts.get(1).unwrap_or(&"").to_string();
                    let fstype = parts.get(2).unwrap_or(&"9p").to_string();
                    let data = parts
                        .get(3)
                        .unwrap_or(&"trans=virtio,version=9p2000.L")
                        .to_string();
                    Mount::new(source, target, fstype, data)
                })
                .collect(),
        )
    }
}

impl Mounts {
    pub fn mount(&self) -> Result<(), Box<dyn std::error::Error>> {
        for mount in &self.0 {
            println!(
                "[MIA] mount: {}:{}:{}:{}",
                &mount.source, &mount.target, &mount.fstype, &mount.data
            );
            mount.mount()?;
        }
        Ok(())
    }
}
