use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
#[derive(Deserialize, Debug)]
struct APIResponse<T> {
    api_version: String,
    method: String,
    fetched: String,
    data: Option<T>,
    error: Option<APIErrorFormat>,
}

#[allow(dead_code)]
/// DMOJ API data format for a single object
#[derive(Deserialize, Debug)]
struct APISingleData<T> {
    object: T,
}

#[allow(dead_code)]
/// DMOJ API data format for lists of objects
#[derive(Deserialize, Debug)]
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
#[derive(Deserialize, Debug)]
struct APIErrorFormat {
    code: i32,
    message: String,
}

#[allow(dead_code)]
/// DMOJ API /api/v2/languages format
#[derive(Deserialize, Debug)]
struct APILanguage {
    id: i32,
    key: String,
    short_name: Option<String>,
    common_name: String,
    ace_mode_name: String,
    pygments_name: String,
    code_template: String,
}

#[allow(dead_code)]
/// DMOJ API /api/v2/submission/<submission id> format
#[derive(Deserialize, Debug)]
struct APISubmission {
    id: i32,
    problem: String,
    user: String,
    date: String,
    time: Option<f64>,
    memory: Option<f64>,
    points: Option<f64>,
    language: String,
    status: String,
    result: Option<String>,
    case_points: f64,
    case_total: f64,
    cases: Vec<APISubmissionCaseOrBatch>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum APISubmissionCaseOrBatch {
    Case(APISubmissionCase),
    Batch(APISubmissionBatch),
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct APISubmissionCase {
    r#type: String,
    case_id: i32,
    status: String,
    time: f64,
    memory: f64,
    points: f64,
    total: f64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct APISubmissionBatch {
    r#type: String,
    batch_id: i32,
    cases: Vec<APISubmissionCase>,
    points: f64,
    total: f64,
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

            // make a map of language keys to language ids
            let key_id_map = get_languages()?
                .into_iter()
                .map(|lang| (lang.key.to_lowercase(), lang.id))
                .collect::<HashMap<String, i32>>();

            let header = format!("Bearer {}", token);
            let url = format!("{}/problem/{}/submit", BASE_URL.to_string(), problem);
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
            // Need some concurrency primitives here to appease the compiler
            let redirect_url = Arc::new(Mutex::new(None));
            let client = {
                let redirect_url_clone = Arc::clone(&redirect_url);
                reqwest::blocking::Client::builder()
                    .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                        *redirect_url_clone.lock().unwrap() = Some(attempt.url().clone());
                        attempt.stop()
                    }))
                    .build()
            }?;
            log::info!("Fetching {} ...", url);
            let submission = client
                .post(&url)
                .form(&params)
                .header(AUTHORIZATION, &header)
                .send()?;

            let redirect_url = redirect_url.lock().unwrap().clone().with_context(|| {
                "Submission request did not get redirected to the submission page"
            })?;
            let res = submission.status().as_u16();
            // TODO: figure out wonkiness with POST codes to make sure it does not break the below code block
            if res != 302 {
                return match res {
                    400 => Err(anyhow!("Error 400, bad request, the header you provided is invalid")),
                    401 => Err(anyhow!("Error 401, unauthorized, the token you provided is invalid")),
                    403 => Err(anyhow!("Error 403, forbidden, you are trying to access the admin portion of the site")),
                    404 => Err(anyhow!("Error 404, not found, the problem does not exist")),
                    500 => Err(anyhow!("Error 500, internal server error")),
                    code => Err(anyhow!("Code {code}, unknown network error")),
                };
            }
            log::info!("submission url: {}", redirect_url);
            let submission_id = redirect_url
                .as_str()
                .split('/')
                .last()
                .with_context(|| "could not determine submission id")?;
            log::info!("submission id: {}", submission_id);

            let client = reqwest::blocking::Client::new();
            loop {
                // TODO: add more logging
                let json: APIResponse<APISingleData<APISubmission>> = client
                    .get(format!("{BASE_URL}/api/v2/submission/{submission_id}"))
                    .header(AUTHORIZATION, &header)
                    .send()?
                    .json()
                    .with_context(|| "converting API response to json failed")?;
                // TODO: maybe add a dmoj_json_unwrap function that encapsulates the
                // if let Some(error) = json.error ... else if let Some(data) = json.data ... else return err
                // form and just returns a Result with successful data.
                // Right now this form is copied/repeated in get_languages.
                if let Some(error) = json.error {
                    return Err(anyhow!(
                        "API request failed with code {} and message `{}`",
                        error.code,
                        error.message
                    ));
                } else if let Some(data) = json.data {
                    // TODO: https://github.com/DMOJ/online-judge/blob/master/judge/models/submission.py
                    //       Use mappings in DMOJ source code to make the messages below actually readable.
                    //       Right now status is stuff like "P" and "G" and result is "AC", "WA", "TLE", etc.
                    if let Some(result) = data.object.result {
                        // Submission has finished grading
                        println!("Submission finished with result {}", result);
                        break;
                    } else {
                        // Submission has not finished grading
                        println!("Status {}", data.object.status);
                    }
                } else {
                    return Err(anyhow!(
                        "Neither data nor error were defined in the API response"
                    ));
                }
                std::thread::sleep(Duration::from_secs(1));
            }
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
