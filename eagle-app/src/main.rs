mod config;

use eagle::engines::VSpec;
use eyre::WrapErr;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(long, short, help = "File path of the configuration toml file")]
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::from_args();
    let content = std::fs::read_to_string(args.config.as_path())
        .wrap_err("Error when reading config file")?;
    let config = toml::de::from_str::<config::Config>(content.as_str())
        .wrap_err("Error when parsing config file")?;

    let process = VSpec::start(config.build()?);

    process.wait_until_complete().await;

    Ok(())
}
