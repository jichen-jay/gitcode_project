pub mod parse_hook;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use std::collections::HashMap;

pub async fn get_files_meta_with_path(
    path_with_namespace: &str,
    pull_number: &str,
) -> anyhow::Result<Vec<FileDetail>> {
    dotenv().ok();
    let access_token =
        std::env::var("access_token").expect("GITHUB_TOKEN env variable is required");

    // https://api.gitcode.com/api/v5/repos/DevCloudFE/vue-devui/pulls/2/files
    let file_list_url = format!(
        "https://api.gitcode.com/api/v5/repos/{}/pulls/{}/files?access_token={}",
        path_with_namespace, pull_number, access_token
    );

    match reqwest::get(&file_list_url).await {
        Ok(res) => {
            let files: Vec<FileChange> = serde_json::from_str(&res.text().await.unwrap()).unwrap();
            let file_details: Vec<FileDetail> = files.into_iter().map(FileDetail::from).collect();
            Ok(file_details)
        }

        Err(e) => Err(anyhow::Error::msg(format!("{:?}", e))),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileDetail {
    pub sha: String,
    pub filename: String,
    pub additions: i32,
    pub deletions: i32,
    pub raw_url: String,
    pub diff: String,
    pub old_path: String,
    pub new_path: String,
    pub new_file: bool,
    pub renamed_file: bool,
    pub deleted_file: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileChange {
    pub sha: String,
    pub filename: String,
    pub additions: i32,
    pub deletions: i32,
    pub raw_url: String,
    pub patch: Patch,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Patch {
    pub diff: String,
    pub old_path: String,
    pub new_path: String,
    pub new_file: bool,
    pub renamed_file: bool,
    pub deleted_file: bool,
}

impl From<FileChange> for FileDetail {
    fn from(file_change: FileChange) -> Self {
        FileDetail {
            sha: file_change.sha,
            filename: file_change.filename,
            additions: file_change.additions,
            deletions: file_change.deletions,
            raw_url: file_change.raw_url,
            diff: file_change.patch.diff,
            old_path: file_change.patch.old_path,
            new_path: file_change.patch.new_path,
            new_file: file_change.patch.new_file,
            renamed_file: file_change.patch.renamed_file,
            deleted_file: file_change.patch.deleted_file,
        }
    }
}

pub async fn down_file_from_raw_url(raw_url: &str) -> anyhow::Result<String> {
    // dotenv().ok();
    // let access_token =
    //     std::env::var("access_token").expect("GITHUB_TOKEN env variable is required");

    // https://raw.gitcode.com/DevCloudFE/vue-devui/raw/8319018d8eba18ebb2923314842da80eeebba6f1/packages/devui-vue/devui/editor-md/src/composables/use-editor-md-toolbar.ts

    match reqwest::get(raw_url).await {
        Ok(content) => Ok(content.text().await.unwrap()),
        Err(e) => Err(anyhow::anyhow!(format!(
            "error downloading file content: {}",
            e
        ))),
    }
}

pub async fn post_on_pr(path_with_namespace: &str, pull_number: &str, body: &str) -> anyhow::Result<String> {
    dotenv().ok();
    let access_token =
        std::env::var("access_token").expect("GITHUB_TOKEN env variable is required");

    let client = reqwest::Client::new();

    // https://api.gitcode.com/api/v5/repos/{{path_with_namespace}}/pulls/2/comments?access_token={{GitCodeNew}}
    let raw_url = format!("https://api.gitcode.com/api/v5/repos/{}/pulls/{}/comments?access_token={}
", path_with_namespace, pull_number, access_token);

    let mut map = HashMap::new();
    map.insert("body", body);

    match client.post(&raw_url).json(&map).send().await {
        Ok(content) => Ok(content.text().await.unwrap()),
        Err(e) => Err(anyhow::anyhow!(format!(
            "error downloading file content: {}",
            e
        ))),
    }
}