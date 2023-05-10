use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity,
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
    /// File extension -> language key mapping, e.g. `cpp:cpp20,py:pypy3,java:java8`
    #[arg(short, long)]
    language: Option<String>,
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

#[derive(Serialize, Deserialize, Default)]
struct ConfyConfig {
    /// API token
    token: Option<String>,
    /// File extension -> language key mapping
    ext_key_map: Option<HashMap<String, String>>,
}

#[allow(dead_code)]
/// DMOJ API response
#[derive(Deserialize)]
struct APIResponse<T> {
    api_version: String,
    method: String,
    fetched: String,
    data: Option<T>,
    error: Option<APIErrorFormat>,
}

#[allow(dead_code)]
/// DMOJ API data format for a single object
#[derive(Deserialize)]
struct APISingleData<T> {
    object: T,
}

#[allow(dead_code)]
/// DMOJ API data format for lists of objects
#[derive(Deserialize)]
struct APIListData<T> {
    current_object_count: i32,
    objects_per_page: i32,
    total_objects: i32,
    page_index: i32,
    total_pages: i32,
    has_more: bool,
    objects: Vec<T>,
}

#[allow(dead_code)]
/// DMOJ API error format
#[derive(Deserialize)]
struct APIErrorFormat {
    code: String,
    message: String,
}

#[allow(dead_code)]
/// DMOJ API /api/v2/languages format
#[derive(Deserialize)]
struct APILanguage {
    id: i32,
    key: String,
    short_name: Option<String>,
    common_name: String,
    ace_mode_name: String,
    pygments_name: String,
    code_template: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    const CONFY_APP_NAME: &str = "dmoj-submit";
    const CONFY_CONFIG_NAME: &str = "config";
    const BASE_URL: &str = "https://dmoj.ca";
    match cli.command {
        Commands::Config(conf_args) => {
            let mut cfg: ConfyConfig = confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME)
                .with_context(|| "could not load configuration")?;
            if let Some(token) = conf_args.token {
                log::info!("setting token to '{}'", token);
                cfg.token = Some(token);
            }
            if let Some(language) = conf_args.language {
                if cfg.ext_key_map.is_none() {
                    cfg.ext_key_map = Some(HashMap::new());
                }
                // split by `,` then split by `:` then insert the resulting pairs into hashmap
                language
                    .split(',')
                    .map(|pair| match pair.split(':').collect::<Vec<&str>>()[..] {
                        [ext, key] => Some((ext, key)),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .with_context(|| "couldn't parse language argument")?
                    .into_iter()
                    .for_each(|(ext, lang_key)| {
                        log::info!("Set extension {ext} to language key {lang_key}");
                        cfg.ext_key_map
                            .as_mut()
                            .unwrap()
                            .insert(ext.to_string(), lang_key.to_string());
                    });
            }
            confy::store(CONFY_APP_NAME, CONFY_CONFIG_NAME, cfg)
                .with_context(|| "could not store configuration")?;
        }
        Commands::Submit(sub_args) => {
            // check that provided file exists
            if !sub_args.file.exists() { return Err(anyhow!("could not find file {}", sub_args.file.display())); }

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
                // need to know what config file struct will look like prior to being able to properly implement this
                // TODO: get submission language from file extension (provided below)
                sub_args
                    .file
                    .extension()
                    .with_context(|| "no file extension specified")?
                    .to_str()
                    .with_context(|| "file extension is not valid Unicode")?
                    .to_string()
            };
            log::info!(
                "Submitting to problem {} with file {}, token `{}`, and language {}",
                problem,
                sub_args.file.display(),
                token,
                language
            );
            // TODO: implement submit function
            
            let client = reqwest::blocking::Client::new();
            let header = format!("Bearer {}", token);
            let url = format!("{}/problem/{}/submit", BASE_URL.to_string(), problem);
            println!("Fetching {} ...", url);
            // TODO: come up with a better variable name than "temp" also consider turning "Authorization" into a const
            let temp = client.get(url).header("Authorization", header).send()?;
            let res = temp.status().as_u16();
            match res {
                // may want to add cases such as 500s for example
                // TODO: specify behavior rather than just having it print out the HTTP status
                200 => println!("200, all good"),
                400 => println!("400, bad request, your header is no good"),
                401 => println!("401, unauthorized, your token is no good"),
                403 => println!("403, forbidden, you are trying to access the admin portion of the site"),
                404 => println!("404, not found, the problem does not exist"),
                _ => println!("reaching this case shouldn't be possible")                
            }
        }
        Commands::ListLanguages => {
            let json: APIResponse<APIListData<APILanguage>> =
                reqwest::blocking::get(format!("{BASE_URL}/api/v2/languages"))
                    .with_context(|| "API request failed")?
                    .json()
                    .with_context(|| "converting API request to json failed")?;
            if let Some(error) = json.error {
                return Err(anyhow!(
                    "API request failed with code {} and message `{}`",
                    error.code,
                    error.message
                ));
            } else if let Some(data) = json.data {
                if data.has_more {
                    // TODO: fix this
                    log::error!("There is more than one page of languages, but we are only reading the first one");
                }
                let mut print_lang_list = data
                    .objects
                    .iter()
                    .map(|lang| format!("{}: {}", lang.common_name, lang.key.to_lowercase()))
                    .collect::<Vec<String>>();
                print_lang_list.sort_unstable();
                println!("{}", print_lang_list.join("\n"));
            } else {
                return Err(anyhow!(
                    "Neither data nor error were defined in the API response"
                ));
            }
        }
    };
    Ok(())
}
