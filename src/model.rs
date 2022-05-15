use crate::{PluginResponse, PluginSearchResult};
use octocrab::models::Repository;

#[derive(Debug)]
pub struct GithubResult {
    pub name: String,
    pub description: String,
    pub uri: String,
}

impl From<Repository> for GithubResult {
    fn from(repository: Repository) -> Self {
        GithubResult {
            name: repository.full_name.unwrap_or(repository.name),
            description: repository.description.unwrap_or_default(),
            uri: repository
                .html_url
                .map(|uri| uri.to_string())
                .unwrap_or_default(),
        }
    }
}

impl GithubResult {
    pub(super) fn to_plugin_response(&self, idx: usize) -> PluginResponse {
        PluginResponse::Append(PluginSearchResult {
            id: idx as u32,
            name: self.name.clone(),
            description: self.description.clone(),
            keywords: None,
            icon: None,
            exec: None,
            window: None,
        })
    }
}
