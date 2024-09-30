
use gitcode_project::{down_file_from_raw_url, get_files_meta_with_path, post_on_pr};
use flowsnet_platform_sdk::logger;
use serde::{Serialize, Deserialize};
use llmservice_flows::{chat::ChatOptions, LLMServiceFlows};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use tokio;
use webhook_flows::{create_endpoint, request_handler};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler]
async fn handler(
    _headers: Vec<(String, String)>,
    _subpath: String,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    logger::init();

    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct Load {
        pub path_with_namespace: String,
        pub pull_number: String,
        pub title: String,
    }

    let load: Load = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            panic!("failed to parse body");
        }
    };

    let path_with_namespace = load.path_with_namespace.clone();
    let pull_number = load.pull_number.clone();
    let title = load.title.clone();

    log::info!("path: {:?}", path_with_namespace);
    log::info!("pull number: {:?}", pull_number);
    log::info!("title: {:?}", title);

    let _ = inner(&path_with_namespace, &pull_number, &title).await;

}

pub async fn inner(path_with_namespace: &str, pull_number: &str, title: &str) -> anyhow::Result<()> {
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
    let llm_api_key = env::var("OPENAI_API_KEY").unwrap_or("LLAMAEDGE".to_string());

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

    let _ = post_on_pr("jaykchen/explore-gc", "2", &resp).await;
    // let _ = post_on_pr(path_with_namespace, pull_number, &resp).await;

    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
