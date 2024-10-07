pub mod parse_hook;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use std::collections::HashMap;
use llmservice_flows::{chat::ChatOptions, LLMServiceFlows};
use std::env;

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

pub async fn fetch_and_review_files(path_with_namespace: &str, pull_number: &str, title: &str) -> anyhow::Result<String> {
    let file_list = get_files_meta_with_path(&path_with_namespace, pull_number)
        .await
        .expect("failed to get files_meta from url");
    match serde_json::to_string_pretty(&file_list) {
        Ok(pretty_json) => log::info!("file list: {}", pretty_json),
        Err(e) => log::error!("Failed to serialize file list: {}", e),
    }

    let llm_api_endpoint =
        env::var("llm_api_endpoint").unwrap_or("https://api.openai.com/v1".to_string());
    let llm_model_name = env::var("llm_model_name").unwrap_or("gpt-4o".to_string());
    let llm_ctx_size = env::var("llm_ctx_size")
        .unwrap_or("16384".to_string())
        .parse::<u32>()
        .unwrap_or(0);
    let llm_api_key = env::var("llm_api_key").unwrap_or("LLAMAEDGE".to_string());

    //  The soft character limit of the input context size
    //  This is measured in chars. We set it to be 2x llm_ctx_size, which is measured in tokens.
    let ctx_size_char: usize = (2 * llm_ctx_size).try_into().unwrap_or(0);

    let chat_id = format!("PR#{pull_number}");

    let system = &format!("You are an experienced software developer. You will review a source code file and its patch related to the subject of \"{}\". Please be as concise as possible while being accurate.", title);
    let mut lf = LLMServiceFlows::new(&llm_api_endpoint);
    lf.set_api_key(&llm_api_key);
    let mut resp = String::new();
    resp.push_str("Hello, I am a [code review agent](https://github.com/flows-network/github-pr-review/) on [flows.network](https://flows.network/). Here are my reviews of changed source code files in this PR.\n\n------\n\n");

    for f in file_list {

        let filename = &f.filename;
        if filename.ends_with(".md")
            || filename.ends_with(".js")
            || filename.ends_with(".css")
            || filename.ends_with(".html")
            || filename.ends_with(".htm")
        {
            continue;
        }

        let file_as_text = down_file_from_raw_url(&f.raw_url)
            .await
            .expect("failed to download file content");
        match serde_json::to_string_pretty(&file_as_text) {
            Ok(pretty_json) => log::info!("content: {}", pretty_json),
            Err(e) => log::error!("Failed to serialize file list: {}", e),
        }

        let t_file_as_text = truncate(&file_as_text, ctx_size_char);

        resp.push_str("## [");
        resp.push_str(filename);
        resp.push_str("](");
        resp.push_str(f.raw_url.as_str());
        resp.push_str(")\n\n");

        log::debug!("Sending file to LLM: {}", filename);
        let co = ChatOptions {
            model: Some(&llm_model_name),
            token_limit: llm_ctx_size,
            restart: true,
            system_prompt: Some(system),
            ..Default::default()
        };
        let question = "Review the following source code and report only major bugs or issues. The most important coding issues should be reported first. You should report NO MORE THAN 3 issues. Be very concise and explain each coding issue in one sentence. The code might be truncated. NEVER comment on the completeness of the source code.\n\n".to_string() + t_file_as_text;
        match lf.chat_completion(&chat_id, &question, &co).await {
            Ok(r) => {
                resp.push_str("#### Potential issues");
                resp.push_str("\n\n");
                resp.push_str(&r.choice);
                resp.push_str("\n\n");
                log::debug!("Received LLM resp for file: {}", filename);
            }
            Err(e) => {
                resp.push_str("#### Potential issues");
                resp.push_str("\n\n");
                resp.push_str("N/A");
                resp.push_str("\n\n");
                log::error!("LLM returns error for file review for {}: {}", filename, e);
            }
        }

        log::debug!("Sending patch to LLM: {}", filename);
        let co = ChatOptions {
            model: Some(&llm_model_name),
            token_limit: llm_ctx_size,
            restart: true,
            system_prompt: Some(system),
            ..Default::default()
        };
        let diff_as_text = f.diff;
        let t_diff_as_text = truncate(&diff_as_text, ctx_size_char);
        let question = "The following is a diff file. Please summarize key changes in short bullet points. List the most important changes first. You list should contain no more than the top 3 most important changes.  \n\n".to_string() + t_diff_as_text;
        match lf.chat_completion(&chat_id, &question, &co).await {
            Ok(r) => {
                resp.push_str("#### Summary of changes");
                resp.push_str("\n\n");
                resp.push_str(&r.choice);
                resp.push_str("\n\n");
                log::debug!("Received LLM resp for patch: {}", filename);
            }
            Err(e) => {
                resp.push_str("#### Summary of changes");
                resp.push_str("\n\n");
                resp.push_str("N/A");
                resp.push_str("\n\n");
                log::error!("LLM returns error for patch review for {}: {}", filename, e);
            }
        }
    }

    log::info!("review: {:?}", resp);

    Ok(resp)
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
