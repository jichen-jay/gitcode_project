use flowsnet_platform_sdk::logger;
use gitcode_project::{fetch_and_review_files, parse_hook::parse_hook_payload, post_on_pr};
use serde_json::Value;
use std::collections::HashMap;
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
    let hook_payload_struct = parse_hook_payload(&_body)
        .await
        .expect("failed to parse payload");

    match serde_json::to_string_pretty(&hook_payload_struct) {
        Ok(pretty_json) => log::info!("Received webhook payload: {}", pretty_json),
        Err(e) => log::error!("Failed to deserialize webhook payload: {}", e),
    }

    let path_with_namespace = hook_payload_struct.project_path_with_namespace;
    let pull_number = hook_payload_struct
        .pull_request_url
        .rsplitn(2, '/')
        .nth(0)
        .unwrap_or("0");
    log::info!("pull_number: {pull_number}");

    let title = hook_payload_struct.title;

    let resp = fetch_and_review_files(&path_with_namespace, &pull_number, &title)
        .await
        .expect("failed to create review");
    let _ = post_on_pr(&path_with_namespace, &pull_number, &resp).await;
}
