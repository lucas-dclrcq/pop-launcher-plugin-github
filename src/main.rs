use std::sync::Arc;
use octocrab::Octocrab;
use pop_launcher::*;

pub struct GithubResult {
    pub name: String,
    pub description: String,
    pub uri: String
}

pub struct SearchContext {
    pub out: tokio::io::Stdout,
    pub search_results: Vec<GithubResult>,
    pub octocrab: Arc<Octocrab>
}

impl Default for SearchContext {
    fn default() -> Self {
        Self  {
            out: async_stdout(),
            search_results: Vec::new(),
            octocrab: octocrab::instance()
        }
    }
}

impl SearchContext {
    async fn activate(&mut self, id: u32) {
        if let Some(result) = self.search_results.get(id as usize) {
            crate::xdg_open(&result.uri);
            crate::send(&mut self.out, PluginResponse::Close).await;
        }
    }

    async fn append(&mut self, id: u32, line: String) {

    }

    async fn search(&mut self, query: String) {
        self.search_results.clear();

        if let Some(word) = query.split_ascii_whitespace().next() {
            match word {
                "gh" => {
                    let page = self.octocrab.search()
                        .repositories(query.to_string())
                        .send()
                        .await?;

                    for repository in page {
                        let github_result = GithubResult {
                            name: repository.name,
                            description: repository.description.unwrap_or_default(),
                            uri: repository.html_url.map().unwrap_or_default(),
                        };
                        self.search_results.(github_result)
                    }
                },
                "pr" => {},
                "repo" => {},
                _ => {}
            }
        }
    }
}

pub fn main() {
    let mut app = SearchContext::default();

    let mut requests = json_input_stream(async_stdin());

    while let Some(result) = requests.next().await {
        match result {
            Ok(request) => match request {
                Request::Activate(id) => app.activate(id).await,
                Request::Search(query) => app.search(query).await,
                Request::Exit => break,
                _ => (),
            },
            Err(why) => tracing::error!("malformed JSON input: {}", why),
        }
    }
}