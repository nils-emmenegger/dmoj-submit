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
    /// Problem code
    problem_code: String,
    /// File to submit
    file: std::path::PathBuf,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    const CONFY_APP_NAME: &str = "dmoj-submit";
    const CONFY_CONFIG_NAME: &str = "config";
    match cli.command {
        Commands::Config(conf_args) => {
            let mut cfg: ConfyConfig = confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME)?;
            if let Some(token) = conf_args.token {
                // TODO: add --verbose option so stuff like this doesn't show up by default
                println!("Setting token to `{}`", token);
                cfg.token = Some(token);
            }
            confy::store(CONFY_APP_NAME, CONFY_CONFIG_NAME, cfg)?;
        }
        Commands::Submit(sub_args) => {
            println!(
                "Submitting to problem {} with file `{}`",
                sub_args.problem_code,
                sub_args.file.display()
            );
            // TODO: get token and language from optional args or config
            // TODO: implement submit function
        }
    };
    Ok(())
}
