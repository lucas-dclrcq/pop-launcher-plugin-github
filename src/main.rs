use std::sync::Arc;
use octocrab::Octocrab;
use pop_launcher_toolkit::launcher;
use pop_launcher_toolkit::launcher::{json_input_stream, Request};
use pop_launcher_toolkit::plugins::{send, xdg_open};
use tokio_stream::StreamExt;
use model::GithubResult;
use crate::config::PluginConfig;
use crate::launcher::{async_stdin, async_stdout, PluginResponse, PluginSearchResult};

mod model;
mod config;

pub struct SearchContext {
    pub out: tokio::io::Stdout,
    pub search_results: Vec<GithubResult>,
    pub octocrab: Arc<Octocrab>
}

impl Default for SearchContext {
    fn default() -> Self {
        let config = PluginConfig::load();
        Self {
            out: async_stdout(),
            search_results: Vec::new(),
            octocrab: Arc::new(Octocrab::builder()
                .personal_token(config.personal_access_token).build()
                                   .expect("Failed to build octocrab client")),
        }
    }
}

impl SearchContext {
    async fn activate(&mut self, id: u32) {
        if let Some(result) = self.search_results.get(id as usize) {
            xdg_open(&result.uri);
            send(&mut self.out, PluginResponse::Close).await;
        }
    }

    async fn search(&mut self, query: String) {
        self.search_results.clear();

        if let Some((command, query)) = query.split_once(' ') {
            match command {
                "gh" => {
                    if query.len() > 4 {

                    }
                    let page = self.octocrab.search()
                        .repositories(&query.to_string())
                        .send()
                        .await
                        .unwrap();

                    self.search_results = page.items.into_iter().map(GithubResult::from).collect();
                    let results = self.search_results.iter()
                        .enumerate()
                        .map(|(idx, search_result)| search_result.into_plugin_response(idx));

                    for search_result in results {
                        send(&mut self.out, search_result).await;
                    }

                    send(&mut self.out, PluginResponse::Finished).await;
                },
                "pr" => {},
                "repo" => {},
                _ => {}
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
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
            Err(why) => eprintln!("malformed JSON input: {}", why),
        }
    }
}