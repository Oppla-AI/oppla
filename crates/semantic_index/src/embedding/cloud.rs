use crate::{Embedding, EmbeddingProvider, TextToEmbed};
use anyhow::{anyhow, Context as _, Result};
use futures::{AsyncReadExt as _, FutureExt, future::BoxFuture};
use http_client::{HttpClient, HttpClientWithUrl, AsyncBody, Method, Request};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use client::Client;
use language_model::LlmApiToken;

pub struct CloudEmbeddingProvider {
    http_client: Arc<HttpClientWithUrl>,
    model: String,
    llm_api_token: LlmApiToken,
    client: Arc<Client>,
}

impl CloudEmbeddingProvider {
    pub fn new(
        http_client: Arc<HttpClientWithUrl>,
        model: String,
        llm_api_token: LlmApiToken,
        client: Arc<Client>,
    ) -> Self {
        Self {
            http_client,
            model,
            llm_api_token,
            client,
        }
    }
}

#[derive(Serialize)]
struct CloudEmbeddingRequest<'a> {
    model: String,
    input: Vec<&'a str>,
}

#[derive(Deserialize)]
struct CloudEmbeddingResponse {
    data: Vec<CloudEmbedding>,
}

#[derive(Deserialize)]
struct CloudEmbedding {
    embedding: Vec<f32>,
}

impl EmbeddingProvider for CloudEmbeddingProvider {
    fn embed<'a>(&'a self, texts: &'a [TextToEmbed<'a>]) -> BoxFuture<'a, Result<Vec<Embedding>>> {
        let model = self.model.clone();
        let http_client = self.http_client.clone();
        let llm_api_token = self.llm_api_token.clone();
        let client = self.client.clone();
        
        async move {
            // Acquire the JWT token
            let token = llm_api_token.acquire(&client).await
                .context("Failed to acquire LLM API token")?;
            
            // Build the URL using build_zed_llm_url
            let url = http_client
                .build_zed_llm_url("/embeddings", &[])
                .context("Failed to build embedding URL")?;
            
            // Prepare the request
            let request = CloudEmbeddingRequest {
                model,
                input: texts.iter().map(|t| t.text).collect(),
            };
            
            let body = serde_json::to_string(&request)
                .context("Failed to serialize embedding request")?;
            
            // Build HTTP request with authentication
            let http_request = Request::builder()
                .method(Method::POST)
                .uri(url.as_str())
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(AsyncBody::from(body))
                .context("Failed to build HTTP request")?;
            
            // Send the request
            let mut response = http_client.send(http_request).await
                .context("Failed to send embedding request")?;
            
            // Check status
            if !response.status().is_success() {
                let mut body = String::new();
                response.body_mut().read_to_string(&mut body).await?;
                return Err(anyhow!(
                    "Embedding request failed with status {}: {}",
                    response.status(),
                    body
                ));
            }
            
            // Parse response
            let mut body = String::new();
            response.body_mut().read_to_string(&mut body).await
                .context("Failed to read response body")?;
            
            let response: CloudEmbeddingResponse = serde_json::from_str(&body)
                .context("Failed to parse embedding response")?;
            
            // Convert to Embedding type
            let embeddings = response.data
                .into_iter()
                .map(|data| Embedding::new(data.embedding))
                .collect();
            
            Ok(embeddings)
        }
        .boxed()
    }
    
    fn batch_size(&self) -> usize {
        // Conservative batch size for cloud API
        100
    }
}