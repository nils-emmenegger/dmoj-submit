use clap::{Args, Parser, Subcommand};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Config(conf_args) => {
            if let Some(token) = conf_args.token {
                println!("Setting token to {}", token);
            }
            // TODO: make a config and set the arguments
            Ok(())
        }
        Commands::Submit(sub_args) => {
            println!(
                "Submitting to problem {} with file {}",
                sub_args.problem_code,
                sub_args.file.display()
            );
            // TODO: get token and language from optional args or config
            // TODO: implement submit function
            Ok(())
        }
    }
}
