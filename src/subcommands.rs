use crate::api::*;
use anyhow::{anyhow, Context, Result};
use reqwest::header::AUTHORIZATION;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn submit(problem: &str, source: &str, token: &str, language: &str) -> Result<()> {
    // make a map of language keys to language ids
    let key_id_map = get_languages()?
        .into_iter()
        .map(|lang| (lang.key.to_lowercase(), lang.id))
        .collect::<HashMap<String, i32>>();
    let lang_id = key_id_map
        .get(&language.to_lowercase())
        .with_context(|| "could not determine language id")?;

    let header = format!("Bearer {}", token);
    let url = format!("{}/problem/{}/submit", BASE_URL, problem);
    let params = [
        ("problem", problem),
        ("source", source),
        ("language", &lang_id.to_string()),
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

    let redirect_url = redirect_url
        .lock()
        .unwrap()
        .clone()
        .with_context(|| "Submission request did not get redirected to the submission page")?;
    let res = submission.status().as_u16();
    // TODO: figure out wonkiness with POST codes to make sure it does not break the below code block
    if res != 302 {
        return match res {
            400 => Err(anyhow!(
                "Error 400, bad request, the header you provided is invalid"
            )),
            401 => Err(anyhow!(
                "Error 401, unauthorized, the token you provided is invalid"
            )),
            403 => Err(anyhow!(
                "Error 403, forbidden, you are trying to access the admin portion of the site"
            )),
            404 => Err(anyhow!("Error 404, not found, the problem does not exist")),
            500 => Err(anyhow!("Error 500, internal server error")),
            code => Err(anyhow!("Code {}, unknown network error", code)),
        };
    }
    log::info!("submission url: {}", redirect_url);
    let submission_id = redirect_url
        .as_str()
        .split('/')
        .last()
        .with_context(|| "could not determine submission id")?;
    log::info!("submission id: {}", submission_id);

    // https://github.com/DMOJ/online-judge/blob/master/judge/models/submission.py
    // from https://github.com/DMOJ/online-judge/blob/81f3c90ffad12586f9edc9e17c8aa0bd66f28ecc/judge/models/submission.py#L49
    const USER_DISPLAY_CODES_TUPLES: [(&str, &str); 15] = [
        ("AC", "Accepted"),
        ("WA", "Wrong Answer"),
        ("SC", "Short Circuited"),
        ("TLE", "Time Limit Exceeded"),
        ("MLE", "Memory Limit Exceeded"),
        ("OLE", "Output Limit Exceeded"),
        ("IR", "Invalid Return"),
        ("RTE", "Runtime Error"),
        ("CE", "Compile Error"),
        ("IE", "Internal Error (judging server error)"),
        ("QU", "Queued"),
        ("P", "Processing"),
        ("G", "Grading"),
        ("D", "Completed"),
        ("AB", "Aborted"),
    ];
    let user_display_codes_map: HashMap<String, String> = HashMap::from_iter(
        USER_DISPLAY_CODES_TUPLES
            .into_iter()
            .map(|(key, val)| (key.to_string(), val.to_string())),
    );
    let get_display = |code: &str| {
        user_display_codes_map
            .get(code)
            .with_context(|| "unknown display code")
    };

    let client = reqwest::blocking::Client::new();
    loop {
        // TODO: add more logging
        let json: APIResponse<APISingleData<APISubmission>> = client
            .get(format!("{}/api/v2/submission/{}", BASE_URL, submission_id))
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
            if let Some(result) = data.object.result {
                // Submission has finished grading
                println!(
                    "Submission finished with result {}",
                    get_display(result.as_str())?
                );
                break;
            } else {
                // Submission has not finished grading
                println!("Status {}", get_display(data.object.status.as_str())?);
            }
        } else {
            return Err(anyhow!(
                "Neither data nor error were defined in the API response"
            ));
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

pub fn list_languages() -> Result<()> {
    let mut print_lang_list = get_languages()?
        .into_iter()
        .map(|lang| format!("{}: {}", lang.common_name, lang.key.to_lowercase()))
        .collect::<Vec<String>>();
    print_lang_list.sort_unstable();
    println!("{}", print_lang_list.join("\n"));
    Ok(())
}
