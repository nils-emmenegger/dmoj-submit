use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

pub const BASE_URL: &str = "https://dmoj.ca";

#[allow(dead_code)]
/// DMOJ API response
#[derive(Deserialize, Debug)]
pub struct APIResponse<T> {
    pub api_version: String,
    pub method: String,
    pub fetched: String,
    pub data: Option<T>,
    pub error: Option<APIErrorFormat>,
}

#[allow(dead_code)]
/// DMOJ API data format for a single object
#[derive(Deserialize, Debug)]
pub struct APISingleData<T> {
    pub object: T,
}

#[allow(dead_code)]
/// DMOJ API data format for lists of objects
#[derive(Deserialize, Debug)]
pub struct APIListData<T> {
    pub current_object_count: i32,
    pub objects_per_page: i32,
    pub total_objects: i32,
    pub page_index: i32,
    pub total_pages: i32,
    pub has_more: bool,
    pub objects: Vec<T>,
}

#[allow(dead_code)]
/// DMOJ API error format
#[derive(Deserialize, Debug)]
pub struct APIErrorFormat {
    pub code: i32,
    pub message: String,
}

#[allow(dead_code)]
/// DMOJ API /api/v2/languages format
#[derive(Deserialize, Debug)]
pub struct APILanguage {
    pub id: i32,
    pub key: String,
    pub short_name: Option<String>,
    pub common_name: String,
    pub ace_mode_name: String,
    pub pygments_name: String,
    pub code_template: String,
}

#[allow(dead_code)]
/// DMOJ API /api/v2/submission/<submission id> format
#[derive(Deserialize, Debug)]
pub struct APISubmission {
    pub id: i32,
    pub problem: String,
    pub user: String,
    pub date: String,
    pub time: Option<f64>,
    pub memory: Option<f64>,
    pub points: Option<f64>,
    pub language: String,
    pub status: String,
    pub result: Option<String>,
    pub case_points: f64,
    pub case_total: f64,
    pub cases: Vec<APISubmissionCaseOrBatch>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum APISubmissionCaseOrBatch {
    Case(APISubmissionCase),
    Batch(APISubmissionBatch),
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct APISubmissionCase {
    pub r#type: String,
    pub case_id: i32,
    pub status: String,
    pub time: f64,
    pub memory: f64,
    pub points: f64,
    pub total: f64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct APISubmissionBatch {
    pub r#type: String,
    pub batch_id: i32,
    pub cases: Vec<APISubmissionCase>,
    pub points: f64,
    pub total: f64,
}

pub fn get_languages() -> Result<Vec<APILanguage>> {
    let json: APIResponse<APIListData<APILanguage>> =
        reqwest::blocking::get(format!("{}/api/v2/languages", BASE_URL))
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
