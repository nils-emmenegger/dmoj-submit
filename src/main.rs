mod api;
mod cli;
mod config;
mod subcommands;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::{get_config, get_config_path, set_config};
use std::{collections::HashMap, fs};

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

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
        Commands::SetConfig(conf_args) => {
            let mut cfg = get_config()?;
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
                        log::info!("Set extension {} to language key {}", ext, lang_key);
                        cfg.ext_key_map
                            .as_mut()
                            .unwrap()
                            .insert(ext.to_string(), lang_key.to_string());
                    });
            }
            set_config(cfg)?;
        }
        Commands::GetConfig => {
            println!("{}", get_config_path()?.display());
            println!("{:#?}", get_config()?);
        }
        Commands::Submit(sub_args) => {
            let source =
                fs::read_to_string(&sub_args.file).with_context(|| "could not read file")?;

            if source.trim().is_empty() {
                return Err(anyhow!("file {} is empty", sub_args.file.display()));
            }

            let cfg = get_config()?;
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
                    log::warn!("Defaulting to {}", default_lang_key);
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
            subcommands::submit(&problem, &source, &token, &language)?;
        }
        Commands::ListLanguages => {
            subcommands::list_languages()?;
        }
    };
    Ok(())
}
