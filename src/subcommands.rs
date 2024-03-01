use crate::api::*;
use anyhow::{anyhow, Context, Result};
use console::style;
use indicatif::ProgressBar;
use reqwest::header::AUTHORIZATION;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use std::{collections::HashMap, sync::Arc};
use APISubmissionCaseOrBatch::{Batch, Case};

struct FlattenedCasesItem {
    /// true if it's a case inside a batch
    is_batched_case: bool,
    /// case or batch number (within a batch, cases start from 1)
    num: i32,
    item: APISubmissionCaseOrBatch,
}

fn flatten_cases(cases: Vec<APISubmissionCaseOrBatch>) -> Vec<FlattenedCasesItem> {
    // `Case`s stay the same, but `Batch`s now contain empty vectors in their `cases` field and the cases that they correspond to follow as `Case`s
    let mut ret = Vec::new();
    let mut batch_num = 1;
    for i in cases.into_iter() {
        match i {
            x @ Case(_) => {
                ret.push(FlattenedCasesItem {
                    is_batched_case: false,
                    // Unbatched cases use and increment the batch number
                    // e.g. https://dmoj.ca/submission/4998420
                    num: batch_num,
                    item: x,
                });
                batch_num += 1;
            }
            Batch(batch) => {
                ret.push(FlattenedCasesItem {
                    is_batched_case: false,
                    num: batch_num,
                    item: Batch(APISubmissionBatch {
                        cases: Vec::new(),
                        ..batch
                    }),
                });
                batch_num += 1;
                ret.extend(batch.cases.into_iter().zip(1..).map(|(case, case_num)| {
                    FlattenedCasesItem {
                        is_batched_case: true,
                        num: case_num,
                        item: Case(case),
                    }
                }));
            }
        }
    }
    ret
}

impl FlattenedCasesItem {
    fn gen_msg(&self) -> String {
        // https://github.com/DMOJ/online-judge/blob/master/templates/submission/status-testcases.html#L51
        match &self.item {
            Case(case) => {
                let case_num = format!("#{}:", self.num);
                // pads the right side with spaces if there are < 5 characters
                // '#' + ':' + up to 3 digits = 5 characters
                let padded_case_num = format!("{:<5}", case_num);
                let title = if self.is_batched_case {
                    style(format!("  Case {}", padded_case_num))
                } else {
                    style(format!("Test case {}", padded_case_num)).bold()
                };
                let status = match case.status.as_str() {
                    "AC" if case.points == case.total => style("AC").green(),
                    "AC" if case.points != case.total => style("AC").yellow().bright(),
                    "WA" => style("WA").red().bright(),
                    "TLE" => style("TLE").black(),
                    "SC" => style("—").black(),
                    code @ ("MLE" | "OLE" | "RTE" | "IR") => style(code).red(),
                    unexpected_status => {
                        log::warn!("Unexpected case status code");
                        style(unexpected_status)
                    }
                };
                // Only used when not SC (short-circuited)
                let time_and_mem =
                    || format!("[{:.3}s, {:.2} MB]", case.time, case.memory / 1024.0);
                // Only used for unbatched test cases
                let points = || format!("({:.0}/{:.0})", case.points, case.total);
                if case.status != "SC" {
                    if self.is_batched_case {
                        format!("{} {} {}", title, status, time_and_mem())
                    } else {
                        format!("{} {} {} {}", title, status, time_and_mem(), points())
                    }
                } else if self.is_batched_case {
                    format!("{} {}", title, status)
                } else {
                    format!("{} {} {}", title, status, points())
                }
            }
            Batch(batch) => {
                let title = style(format!("Batch #{}", self.num)).bold();
                let points = format!("(?/{:.0} points)", batch.total);
                format!("{} {}", title, points)
            }
        }
    }
}

struct Progress {
    spinner: ProgressBar,
    cases: Vec<FlattenedCasesItem>,
}

impl Progress {
    fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(Duration::from_millis(120));
        Self {
            spinner,
            cases: Vec::new(),
        }
    }

    fn extend(&mut self, cases: Vec<APISubmissionCaseOrBatch>) {
        let mut cases = flatten_cases(cases);

        let new_cases = cases.split_off(self.cases.len());
        let _old_cases = cases;

        // print new cases and add to self.cases
        for case in new_cases.into_iter() {
            self.spinner.println(case.gen_msg());
            self.cases.push(case);
        }
    }

    fn finish(self) {
        self.spinner.finish_and_clear();
    }
}

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
    let redirect_url = Arc::new(OnceLock::new());
    let client = {
        let redirect_url_clone = Arc::clone(&redirect_url);
        reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                redirect_url_clone.get_or_init(|| attempt.url().clone());
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
        .get()
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

    let client = reqwest::blocking::Client::new();
    let mut progress = Progress::new();
    loop {
        let before_req = Instant::now();
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
            progress.extend(data.object.cases);

            if let Some(result) = data.object.result {
                // Submission has finished grading
                progress.finish();
                println!();
                // https://github.com/DMOJ/online-judge/blob/master/templates/submission/status-testcases.html#L126
                match result.as_str() {
                    "IE" => {
                        // https://github.com/DMOJ/online-judge/blob/master/templates/submission/internal-error-message.html#L3
                        println!("{}", style("An internal error occurred while grading, and the DMOJ administrators have been notified\nIn the meantime, try resubmitting in a few seconds.").red().bright())
                    }
                    "CE" => println!("Compilation error"),
                    "AB" => println!("Submission aborted!"),
                    _ => {
                        // print resources
                        println!(
                            "{} {}, {:.2} MB",
                            style("Resources:").bold(),
                            if result == "TLE" {
                                "---".to_string()
                            } else {
                                format!("{:.3}s", data.object.time.unwrap())
                            },
                            data.object.memory.unwrap() / 1024.0,
                        );

                        // TODO: implement maximum single-case runtime

                        // print final score
                        println!(
                            "{} {:.0}/{:.0}",
                            style("Final score:").bold(),
                            data.object.case_points,
                            data.object.case_total
                        );
                    }
                }
                break;
            }
        } else {
            return Err(anyhow!(
                "Neither data nor error were defined in the API response"
            ));
        }
        let after_req = Instant::now();
        // 1 second between requests
        // We can subtract the time that the request took
        std::thread::sleep(
            Duration::from_secs(1).saturating_sub(after_req.duration_since(before_req)),
        );
    }
    Ok(())
}

pub fn list_languages() -> Result<()> {
    let mut print_lang_list = get_languages()?
        .into_iter()
        .map(|lang| format!("{}: {}", lang.common_name, lang.key.to_lowercase()))
        .collect::<Vec<String>>();
    print_lang_list.sort_unstable();
    println!(
        "{}: {}",
        style("Common name").underlined().bold(),
        style("Language key").underlined().bold()
    );
    println!("{}", print_lang_list.join("\n"));
    Ok(())
}
