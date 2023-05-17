use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

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

fn get_languages() -> Result<Vec<APILanguage>> {
    let json: APIResponse<APIListData<APILanguage>> =
        reqwest::blocking::get(format!("{BASE_URL}/api/v2/languages"))
            .with_context(|| "API request failed")?
            .json()
            .with_context(|| "converting API response to json failed")?;
    if let Some(error) = json.error {
        Err(anyhow!(
            "API request failed with code {} and message `{}`",
            error.code,
            error.message
        ))
    } else if let Some(data) = json.data {
        if data.has_more {
            // TODO: fix this
            log::error!(
                "There is more than one page of languages, but we are only reading the first one"
            );
        }
        Ok(data.objects)
    } else {
        Err(anyhow!(
            "Neither data nor error were defined in the API response"
        ))
    }
}

const BASE_URL: &str = "https://dmoj.ca";

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    const CONFY_APP_NAME: &str = "dmoj-submit";
    const CONFY_CONFIG_NAME: &str = "config";
    // TODO: add more defaults
    /// file extension -> language key default mapping as array of tuples
    const EXT_KEY_DEFAULT_TUPLES: [(&str, &str); 14] = [
        ("c", "c"),
        ("cpp", "cpp20"),
        ("java", "java"),
        ("kt", "kotlin"),
        ("py", "pypy3"),
        ("lua", "lua"),
        ("rs", "rust"),
        ("txt", "text"),
        ("go", "go"),
        ("hs", "hask"),
        ("js", "v8js"),
        ("nim", "nim"),
        ("ml", "ocaml"),
        ("zig", "zig"),
    ];
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
            if !sub_args.file.is_file() {
                return Err(anyhow!("could not find file {}", sub_args.file.display()));
            }

            let source =
                fs::read_to_string(&sub_args.file).with_context(|| "could not read file")?;

            if source.trim().is_empty() {
                return Err(anyhow!("file {} is empty", sub_args.file.display()));
            }

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
                let file_ext = sub_args
                    .file
                    .extension()
                    .with_context(|| "no file extension specified")?
                    .to_str()
                    .with_context(|| "file extension is not valid Unicode")?
                    .to_string();
                let ext_key_default_map: HashMap<String, String> = HashMap::from_iter(
                    EXT_KEY_DEFAULT_TUPLES
                        .into_iter()
                        .map(|(key, val)| (key.to_string(), val.to_string())),
                );
                if let Some(cfg_lang_key) =
                    cfg.ext_key_map.and_then(|hm| hm.get(&file_ext).cloned())
                {
                    cfg_lang_key
                } else if let Some(default_lang_key) = ext_key_default_map.get(&file_ext).cloned() {
                    log::warn!("Defaulting to {default_lang_key}");
                    default_lang_key
                } else {
                    return Err(anyhow!("could not determine language"));
                }
            };
            log::info!(
                "Submitting to problem {} with file {}, token `{}`, and language {}",
                problem,
                sub_args.file.display(),
                token,
                language
            );
            // Try to access the /problem/{problem}/submit page
            let client = reqwest::blocking::Client::new();
            let header = format!("Bearer {}", token);
            let url = format!("{}/problem/{}/submit", BASE_URL.to_string(), problem);
            println!("Fetching {} ...", url);

            // TODO: figure out what to do about the .to_lowercase spam
            let key_id_map = get_languages()?
                .into_iter()
                .map(|lang| (lang.key.to_lowercase(), lang.id))
                .collect::<HashMap<String, i32>>();
            let params = [
                ("problem", problem),
                ("source", source),
                (
                    "language",
                    key_id_map
                        .get(&language.to_lowercase())
                        .with_context(|| "could not determine language id")?
                        .to_string(),
                ),
            ];
            // TODO: empty file returns status code 200 but does not actually submit or redirect
            // FIXED!, files are check to ensure they are not empty now so this should not be possible.
            //         Still keeping the original comment because successful submissions should return
            //         a 300 error code, not a 200
            let submission = client
                .post(&url)
                .form(&params)
                .header(AUTHORIZATION, &header)
                .send()?;
            let res = submission.status().as_u16();
            // TODO: figure out wonkiness with POST codes to make sure it does not break the below code block
            if res != 200 {
                return match res {
                    400 => Err(anyhow!("Error 400, bad request, the header you provided is invalid")),
                    401 => Err(anyhow!("Error 401, unauthorized, the token you provided is invalid")),
                    403 => Err(anyhow!("Error 403, forbidden, you are trying to access the admin portion of the site")),
                    404 => Err(anyhow!("Error 404, not found, the problem does not exist")),
                    500 => Err(anyhow!("Error 500, internal server error")),
                    code => Err(anyhow!("Code {code}, unknown network error")),
                };
            }

            let submission_id = &submission.url().as_str()[27..];
            println!("submission: {}", submission_id);
            // TODO: monitor submission status using the /api/v2/submission/<submission id> endpoint
        }
        Commands::ListLanguages => {
            let mut print_lang_list = get_languages()?
                .into_iter()
                .map(|lang| format!("{}: {}", lang.common_name, lang.key.to_lowercase()))
                .collect::<Vec<String>>();
            print_lang_list.sort_unstable();
            println!("{}", print_lang_list.join("\n"));
        }
    };
    Ok(())
}
