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
    /// Retrieve available languages from the DMOJ API
    ListLanguages,
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
    const BASE_URL: &str = "https://dmoj.ca";
    match cli.command {
        Commands::Config(conf_args) => {
            let mut cfg: ConfyConfig = confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME)
                .with_context(|| "could not load configuration")?;
            if let Some(token) = conf_args.token {
                // TODO: add --verbose option so stuff like this doesn't show up by default
                println!("Setting token to `{}`", token);
                cfg.token = Some(token);
            }
            confy::store(CONFY_APP_NAME, CONFY_CONFIG_NAME, cfg)
                .with_context(|| "could not store configuration")?;
        }
        Commands::Submit(sub_args) => {
            let cfg: ConfyConfig = confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME)
                .with_context(|| "could not load configuration")?;
            let problem = if let Some(problem) = sub_args.problem {
                problem
            } else {
                // if unspecified, get problem name from file stem
                sub_args
                    .file
                    .file_stem()
                    .with_context(|| "no file name specified")?
                    .to_str()
                    .with_context(|| "file name is not valid Unicode")?
                    .to_string()
            };
            let token = if let Some(token) = sub_args.token {
                token
            } else {
                // if unspecified, get API token from configuration
                cfg.token
                    .with_context(|| "API token not defined in configuration")?
            };
            let language = if let Some(language) = sub_args.language {
                language
            } else {
                // if unspecified, get language from file extension + configuration
                // need to know what config file struct will look like prior to being able to properly implement this, place holder value
                "temp".to_string()
            };
            println!(
                "Submitting to problem {} with file {}, token `{}`, and language {}",
                problem,
                sub_args.file.display(),
                token,
                language
            );
            // TODO: implement submit function
        }
        Commands::ListLanguages => {
            println!(
                "{}",
                reqwest::blocking::get(format!("{BASE_URL}/api/v2/languages"))?.text()?
            );
        }
    };
    Ok(())
}
