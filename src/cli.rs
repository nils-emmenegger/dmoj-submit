use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(flatten)]
    pub verbose: Verbosity,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Set default API token, language, etc.
    Config(ConfigArgs),
    /// Submit to a problem
    Submit(SubmitArgs),
    /// Get available languages from DMOJ and print as `common_name: language_key` pairs
    ListLanguages,
}

#[derive(Args)]
#[group(required = true, multiple = true)]
pub struct ConfigArgs {
    /// Set API token
    #[arg(short, long)]
    pub token: Option<String>,
    /// File extension -> language key mapping, e.g. `cpp:cpp20,py:pypy3,java:java8`
    #[arg(short, long)]
    pub language: Option<String>,
}

#[derive(Args)]
pub struct SubmitArgs {
    /// File to submit
    pub file: std::path::PathBuf,
    /// Problem code
    #[arg(short, long)]
    pub problem: Option<String>,
    /// API token
    #[arg(short, long)]
    pub token: Option<String>,
    /// Submission language
    #[arg(short, long)]
    pub language: Option<String>,
}
