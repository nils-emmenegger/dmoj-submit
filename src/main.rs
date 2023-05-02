use clap::{arg, command, value_parser};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = command!()
        .arg(
            arg!([PATH])
                // This argument is required for the program to run
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(-t - -token[token])
                // This argument is not required for the program to run, so the program may use a default value
                .required(false)
                .default_value("default"),
        )
        .arg(
            arg!(-l - -language[language])
                // This argument is not required for the program to run, so the program may use a default value
                .required(false)
                .default_value("default"),
        )
        .get_matches();

    // check that provided file exists
    if let Some(path) = matches.get_one::<PathBuf>("PATH") {
        let result = std::fs::read_to_string(path);
        match result {
            // TODO: add desired behavior for each case
            Ok(content) => {
                println!("File content: {}", content);
            }
            Err(error) => {
                println!("Error: {}", error);
            }
        }
    }

    // check if token argument has been provided
    if let Some(token) = matches.get_one::<String>("token") {
        if token.eq("default") {
            // TODO: add desired behavior
            println!("default token value");
        }
    }

    if let Some(language) = matches.get_one::<String>("language") {
        if language.eq("default") {
            // TODO: add desired behavior
            println!("default language value");
        }
    }

    Ok(())
}
