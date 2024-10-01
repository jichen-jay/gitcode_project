use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gitcode_project::{fetch_and_review_files, post_on_pr};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
    dotenv().ok();

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

    let resp = fetch_and_review_files(&path_with_namespace, &pull_number, &title)
        .await
        .expect("failed to create review");
    let _ = post_on_pr("jaykchen/explore-gc", "2", &resp).await;
}
