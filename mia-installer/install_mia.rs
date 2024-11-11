use mia_installer::InstallConfig;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli_args = InstallConfig::from_args();
    mia_installer::install(&cli_args)
}
