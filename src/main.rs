use crate::config::PluginConfig;
use crate::launcher::{async_stdin, PluginResponse, PluginSearchResult};
use anyhow::{anyhow, Result};
use model::GithubResult;
use octocrab::Octocrab;
use pop_launcher_toolkit::launcher;
use pop_launcher_toolkit::launcher::{json_input_stream, Request};
use pop_launcher_toolkit::plugins::{send, xdg_open};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Receiver;
use tokio::{join, select};
use tokio_stream::StreamExt;

mod config;
mod model;

type SearchResult = Arc<Mutex<Vec<GithubResult>>>;

// Holds the search results, github client and channel to pass messages around
pub struct SearchContext {
    interrupt_tx: tokio::sync::broadcast::Sender<()>,
    client: Octocrab,
    search_tx: tokio::sync::mpsc::Sender<Vec<GithubResult>>,
    search_results: SearchResult,
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<()> {
    init_logging();
    let (search_tx, mut search_rx) = tokio::sync::mpsc::channel(8);
    let (interrupt_tx, _) = tokio::sync::broadcast::channel(8);
    let search_results = Arc::new(Mutex::new(vec![]));
    let token = PluginConfig::load().personal_access_token;
    let client = Octocrab::builder().personal_token(token).build()?;

    let mut app = SearchContext {
        client,
        interrupt_tx,
        search_tx,
        search_results: Arc::clone(&search_results),
    };

    let _ = join!(
        listen_for_request(&mut app),
        dispatch_search_result(&mut search_rx, search_results)
    );
    Ok(())
}

impl SearchContext {
    async fn activate(&mut self, id: u32) {
        // Wrap the mutex guard into a scope so we don't hold it across the async `send` method.
        let uri = {
            let search_results = self.search_results.lock().unwrap();
            search_results
                .get(id as usize)
                .map(|selected| selected.uri.clone())
        };

        // Open the github url and tell pop-launcher to close
        if let Some(uri) = uri {
            xdg_open(&uri);
            send(&mut tokio::io::stdout(), PluginResponse::Close).await;
        }
    }

    async fn search(&self, query: String) {
        let client = self.client.clone();
        let sender = self.search_tx.clone();
        let mut interrupt = self.interrupt_tx.subscribe();

        let query = query
            .split_once(' ')
            .and_then(|(_command, query)| match _command {
                "gh" => Some(query.to_string()),
                "pr" => Some(query.to_string()),
                "repo" => Some(query.to_string()),
                _ => None,
            });

        if let Some(query) = query {
            // Avoid sending empty requests to github
            if query.trim().is_empty() {
                return;
            }

            // Either we get something from the search request or we get an interrupt request and
            // Return early.
            tokio::spawn(async move {
                select! {
                    query_result = run_query(query, client) => {
                        match query_result {
                            Ok(query_result) => sender.send(query_result).await.expect("Failed to send query result"),
                            Err(why) => tracing::error!("Failed to obtain query result from github: {why}")
                        }
                    }

                    Ok(()) = interrupt.recv() => {
                        // Just return from this future
                    }
                }
            });
        };
    }
}

async fn listen_for_request(app: &mut SearchContext) {
    let mut requests = json_input_stream(async_stdin());

    while let Some(request) = requests.next().await {
        match request {
            Ok(request) => match request {
                Request::Activate(id) => app.activate(id).await,
                Request::Interrupt => {
                    // Interrupt the previous search query if any
                    let _ = app.interrupt_tx.send(());
                    // Clear the search results
                    let mut search_results = app.search_results.lock().unwrap();
                    search_results.clear();
                    // Tell pop-launcher we are done
                    send(&mut tokio::io::stdout(), PluginResponse::Finished).await;
                }
                Request::Search(query) => app.search(query).await,
                Request::Exit => break,
                _ => (),
            },
            Err(why) => tracing::error!("malformed JSON input: {}", why),
        };
    }
}

// Receive message from the search task and dispatch them to pop-launcher
async fn dispatch_search_result(
    search_rx: &mut Receiver<Vec<GithubResult>>,
    search_results: SearchResult,
) {
    while let Some(new_results) = search_rx.recv().await {
        // Wrap the mutex guard into a scope so we don't hold it across the async `send` method.
        let plugin_responses: Vec<PluginResponse> = {
            let mut search_results = search_results.lock().unwrap();
            *search_results = new_results;

            search_results
                .iter()
                .enumerate()
                .map(|(idx, entry)| entry.to_plugin_response(idx))
                .collect()
        };

        for search_result in plugin_responses {
            send(&mut tokio::io::stdout(), search_result).await;
        }

        send(&mut tokio::io::stdout(), PluginResponse::Finished).await;
    }
}

async fn run_query(query: String, client: Octocrab) -> Result<Vec<GithubResult>> {
    let page = client.search().repositories(&query).send().await;

    page.map_err(|err| anyhow!("HTTP error querying github repositories: {err:?}"))
        .map(|page| page.items.into_iter().map(GithubResult::from).collect())
}

fn init_logging() {
    let logdir = match dirs::state_dir() {
        Some(dir) => dir.join("pop-launcher/"),
        None => dirs::home_dir()
            .expect("home directory required")
            .join(".cache/pop-launcher"),
    };

    let _ = std::fs::create_dir_all(&logdir);

    let logfile = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(logdir.join(["github.log"].concat().as_str()).as_path());

    if let Ok(file) = logfile {
        use tracing_subscriber::{fmt, EnvFilter};
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(file)
            .init();
    }
}
