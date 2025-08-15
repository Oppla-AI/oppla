use anyhow::Result;
use client::Client;
use gpui::App;
use http_client::HttpClientWithUrl;
use language_models::LlmApiToken;
use project::Project;
use semantic_index::{CloudEmbeddingProvider, SemanticDb};
use std::{path::Path, path::PathBuf, sync::Arc};

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <project_path>", args[0]);
        std::process::exit(1);
    }

    App::production(Arc::default()).run(async move |cx| {
        // Initialize HTTP client with base URL
        let http = Arc::new(HttpClientWithUrl::new(
            reqwest_client::ReqwestClient::new(),
            "https://app.oppla.ai/home", // This will be mapped to the LLM endpoint
            None,                        // No proxy
        ));

        // Get client and token
        let client = Client::global(cx);
        let llm_api_token = LlmApiToken::default();

        // Create the cloud embedding provider
        // Using Together AI's cheapest embedding tier
        let embedding_provider = Arc::new(CloudEmbeddingProvider::new(
            http.clone(),
            "together-ai-embedding-up-to-150m".to_string(), // Together AI cheapest tier
            llm_api_token,
            client.clone(),
        ));

        cx.spawn(async move |cx| {
            // Initialize semantic index with cloud provider
            let semantic_index = SemanticDb::new(
                PathBuf::from("/tmp/cloud-semantic-index-db.mdb"),
                embedding_provider,
                cx,
            );

            let mut semantic_index = semantic_index.await.unwrap();

            let project_path = Path::new(&args[1]);
            let project = Project::example([project_path], cx).await;

            cx.update(|cx| {
                let language_registry = project.read(cx).languages().clone();
                let node_runtime = project.read(cx).node_runtime().unwrap().clone();
                languages::init(language_registry, node_runtime, cx);
            })
            .unwrap();

            let project_index = cx
                .update(|cx| semantic_index.project_index(project.clone(), cx))
                .unwrap()
                .await
                .unwrap();

            cx.update(|cx| {
                let project_index = project_index.read(cx);
                let query = "function to handle user authentication";
                println!("Searching for: {}", query);
                project_index.search(vec![query.into()], 10, cx)
            })
            .await
            .unwrap()
            .await
            .unwrap();

            println!("Search completed successfully!");
        })
        .await
        .unwrap();
    });
}
