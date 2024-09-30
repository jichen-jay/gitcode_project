use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct HookPayload {
    pub event_type: Option<String>,
    pub git_target_branch_commit_no: String,
    pub action: String,                       // object_attributes.action
    pub email: String,                        // object_attributes.author.email
    pub name: String,                         // object_attributes.author.name
    pub target_branch: String,                // object_attributes.target_branch
    pub target_branch_commit_id: String,      // object_attributes.target_branch_commit.id
    pub target_branch_commit_message: String, // object_attributes.target_branch_commit.message
    pub pull_request_url: String,             // object_attributes.url
    pub target_branch_commit_url: String,     // object_attributes.target_branch_commit.url
    pub title: String,                        // object_attributes.title
    pub repository_git_http_url: String,      // repository.git_http_url
    pub project_path_with_namespace: String,  // project.path_with_namespace
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObjectAttributes {
    pub action: String,
    pub author: Author,
    pub target_branch: String,
    pub target_branch_commit: Commit,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Author {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub git_http_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub path_with_namespace: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FullPayload {
    pub event_type: Option<String>,
    pub git_target_branch_commit_no: String,
    pub object_attributes: ObjectAttributes,
    pub repository: Repository,
    pub project: Project,
}

impl From<FullPayload> for HookPayload {
    fn from(payload: FullPayload) -> Self {
        HookPayload {
            event_type: payload.event_type,
            git_target_branch_commit_no: payload.git_target_branch_commit_no,
            action: payload.object_attributes.action,
            email: payload.object_attributes.author.email,
            name: payload.object_attributes.author.name,
            target_branch: payload.object_attributes.target_branch,
            target_branch_commit_id: payload.object_attributes.target_branch_commit.id,
            target_branch_commit_message: payload.object_attributes.target_branch_commit.message,
            pull_request_url: payload.object_attributes.url,
            target_branch_commit_url: payload.object_attributes.target_branch_commit.url,
            title: payload.object_attributes.title,
            repository_git_http_url: payload.repository.git_http_url,
            project_path_with_namespace: payload.project.path_with_namespace,
        }
    }
}

pub async fn parse_hook_payload(payload: &[u8]) -> anyhow::Result<HookPayload> {
    let full_payload: FullPayload =
        serde_json::from_slice(payload).expect("JSON was not well-formatted");

    let hook_payload: HookPayload = full_payload.into();

    Ok(hook_payload)
}
