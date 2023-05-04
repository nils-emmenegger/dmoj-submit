use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set default API token, language, etc.
    Config(ConfigArgs),
    /// Submit to a problem
    Submit(SubmitArgs),
}

#[derive(Args)]
#[group(required = true, multiple = true)]
struct ConfigArgs {
    /// Set API token
    #[arg(short, long)]
    token: Option<String>,
}

#[derive(Args)]
struct SubmitArgs {
    /// File to submit
    file: std::path::PathBuf,
    /// Problem code
    #[arg(short, long)]
    problem: Option<String>,
    /// API token
    #[arg(short, long)]
    token: Option<String>,
    /// Submission language
    #[arg(short, long)]
    language: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ConfyConfig {
    /// API token
    token: Option<String>,
}

impl Default for ConfyConfig {
    fn default() -> Self {
        Self { token: None }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    const CONFY_APP_NAME: &str = "dmoj-submit";
    const CONFY_CONFIG_NAME: &str = "config";
    match cli.command {
        Commands::Config(conf_args) => {
            let mut cfg: ConfyConfig = confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME)
                .with_context(|| "could not load configuration")?;
            if let Some(token) = conf_args.token {
                /// TODO: add --verbose option so stuff like this doesn't show up by default
                println!("Setting token to `{}`", token);
                cfg.token = Some(token);
            }
            confy::store(CONFY_APP_NAME, CONFY_CONFIG_NAME, cfg)
                .with_context(|| "could not store configuration")?;
        }
        Commands::Submit(sub_args) => {
            let cfg: ConfyConfig = confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME)
                .with_context(|| "could not load configuration")?;
            let problem = if sub_args.problem.is_none() {
                sub_args.file.file_prefix().unwrap()                
            } else {
                sub_args.problem.unwrap()
            };

            let token = if sub_args.token.is_none() {
                cfg.token.unwrap()
            } else {
                sub_args.token.unwrap()
            };

            let language = if sub_args.language.is_none() {
                // need to know what config file struct will look like prior to being able to properly implement this, place holder value
                "temp".to_string()
            } else {
                sub_args.language.unwrap()
            };

            // TODO: get token and language from optional args or config
            // TODO: implement submit function
        }
    };
    Ok(())
}
