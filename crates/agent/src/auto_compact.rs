use anyhow::Result;
use client::Client;
use gpui::{App, AsyncApp, WeakEntity};
use language_model::{LanguageModel, LlmApiToken, Role};
use oppla_llm_client::EXPIRED_LLM_TOKEN_HEADER_NAME;
use serde::{Deserialize, Serialize};
use smol::io::AsyncReadExt;
use std::sync::Arc;
use uuid::Uuid;

use crate::thread::{MessageId, Thread, ThreadId};
use crate::thread_memory::ThreadMemory;

/// Internal tool for automatic thread compression.
/// This tool is NOT exposed to the LLM - it operates behind the scenes.
pub struct AutoCompactTool;

impl AutoCompactTool {
    /// Check if compression is needed based on token count
    pub fn should_compress(thread: &Thread, model: &Arc<dyn LanguageModel>, cx: &App) -> bool {
        // Don't compress if already compressing
        if thread.is_compressing() {
            log::debug!("Compression already in progress, skipping");
            return false;
        }

        // Get current token count
        let total_tokens = thread.estimate_token_count(model, cx);
        let max_tokens = model.max_token_count();
        // TESTING: Lowered to 20% for testing, should be 0.8 in production
        let threshold = (max_tokens as f64 * 0.2) as u64;

        // Log the raw models response exactly as received from server
        log::info!("Threshold what actually has model: {}", max_tokens);
        log::info!("Threshold of model calculated: {}", threshold);

        total_tokens > threshold
    }

    /// Get messages to compress without holding thread reference
    pub fn get_messages_to_compress(thread: &Thread) -> Vec<crate::thread::Message> {
        let compress_up_to_idx = Self::find_compression_boundary_simple(thread);
        if let Some(idx) = compress_up_to_idx {
            thread.messages().take(idx + 1).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Find compression boundary without needing model/cx
    fn find_compression_boundary_simple(thread: &Thread) -> Option<usize> {
        let messages = thread.messages();
        if messages.len() < 3 {
            return None;
        }

        // Compress approximately 50-60% of older messages
        let compress_ratio = 0.6;
        let compress_up_to = (messages.len() as f64 * compress_ratio) as usize;

        // Ensure we keep at least 2-3 recent messages
        let min_keep_recent = 3;
        if messages.len() - compress_up_to < min_keep_recent {
            let compress_up_to = messages.len().saturating_sub(min_keep_recent);
            if compress_up_to == 0 {
                return None;
            }
            return Some(compress_up_to - 1);
        }

        Some(compress_up_to - 1)
    }

    /// Compress messages (called from spawned task)
    pub async fn compress_messages(
        thread: WeakEntity<Thread>,
        messages_to_compress: Vec<crate::thread::Message>,
        client: Arc<Client>,
        model: Arc<dyn LanguageModel>,
        thread_id: ThreadId,
        cx: &mut AsyncApp,
    ) -> Result<()> {
        if messages_to_compress.is_empty() {
            return Ok(());
        }

        let start_message_id = messages_to_compress.first().unwrap().id;
        let end_message_id = messages_to_compress.last().unwrap().id;
        let model_name = model.id().0.to_string();

        // Set compression flag to prevent concurrent compressions
        thread.update(cx, |thread, _cx| {
            thread.set_compressing(true);
        })?;

        log::info!(
            "🔄 Calling compression service for {} messages",
            messages_to_compress.len()
        );
        // Call compression service
        let compression_result = Self::call_compression_service(
            &client,
            &messages_to_compress,
            &model_name,
            &thread_id.to_string(),
        )
        .await;

        match compression_result {
            Ok(response) => {
                log::info!(
                    "✅ Compression successful: {} -> {} tokens ({}% ratio)",
                    response.original_tokens,
                    response.compressed_tokens,
                    (response.compression_ratio * 100.0) as u32
                );
                // Create memory record from response
                let memory = ThreadMemory {
                    id: Uuid::new_v4().to_string(),
                    thread_id: thread_id.to_string(),
                    original_message_ids: messages_to_compress
                        .iter()
                        .map(|m| m.id.to_string())
                        .collect(),
                    start_message_id: start_message_id.to_string(),
                    end_message_id: end_message_id.to_string(),
                    compressed_content: response.compressed_content,
                    summary: response.summary,
                    compression_ratio: response.compression_ratio,
                    original_tokens: response.original_tokens,
                    compressed_tokens: response.compressed_tokens,
                    tokens_saved: response.original_tokens - response.compressed_tokens,
                    user_edited: false,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    edited_at: None,
                    is_active: true,
                    metadata: crate::thread_memory::MemoryMetadata {
                        conversation_range: format!(
                            "Messages {}-{}",
                            start_message_id, end_message_id
                        ),
                        key_topics: Vec::new(),
                        content_type: "general".to_string(),
                        priority: 5,
                    },
                };

                // Store memory and update thread
                thread.update(cx, move |thread, cx| {
                    thread.set_compression_watermark(end_message_id);
                    thread.memory_manager_mut().add_memory(Arc::new(memory));
                    thread.set_compressing(false); // Clear flag
                    cx.notify();
                })?;
            }
            Err(err) => {
                log::error!("Compression failed, using fallback: {}", err);
                // Fall back to mock compression
                let compression_prompt = Self::build_compression_prompt(&messages_to_compress);
                let compressed_content = Self::mock_compress(&compression_prompt);
                let original_tokens = compression_prompt.len() / 4;
                let compressed_tokens = compressed_content.len() / 4;

                let memory = ThreadMemory {
                    id: Uuid::new_v4().to_string(),
                    thread_id: thread_id.to_string(),
                    original_message_ids: messages_to_compress
                        .iter()
                        .map(|m| m.id.to_string())
                        .collect(),
                    start_message_id: start_message_id.to_string(),
                    end_message_id: end_message_id.to_string(),
                    compressed_content: compressed_content.clone(),
                    summary: Self::extract_summary(&compressed_content),
                    compression_ratio: compressed_tokens as f32 / original_tokens as f32,
                    original_tokens,
                    compressed_tokens,
                    tokens_saved: original_tokens - compressed_tokens,
                    user_edited: false,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    edited_at: None,
                    is_active: true,
                    metadata: crate::thread_memory::MemoryMetadata {
                        conversation_range: format!(
                            "Messages {}-{}",
                            start_message_id, end_message_id
                        ),
                        key_topics: Vec::new(),
                        content_type: "general".to_string(),
                        priority: 5,
                    },
                };

                thread.update(cx, move |thread, cx| {
                    thread.set_compression_watermark(end_message_id);
                    thread.memory_manager_mut().add_memory(Arc::new(memory));
                    thread.set_compressing(false); // Clear flag
                    cx.notify();
                })?;
            }
        }

        Ok(())
    }

    /// Find the optimal compression boundary
    /// Returns the index of the last message to compress
    pub fn find_compression_boundary(
        thread: &Thread,
        _model: &Arc<dyn LanguageModel>,
        _cx: &App,
    ) -> Option<usize> {
        let messages = thread.messages();
        if messages.len() < 3 {
            // Don't compress if we have too few messages
            return None;
        }

        // Compress approximately 50-60% of older messages
        let compress_ratio = 0.6;
        let compress_up_to = (messages.len() as f64 * compress_ratio) as usize;

        // Ensure we keep at least 2-3 recent messages
        let min_keep_recent = 3;
        if messages.len() - compress_up_to < min_keep_recent {
            let compress_up_to = messages.len().saturating_sub(min_keep_recent);
            if compress_up_to == 0 {
                return None;
            }
            return Some(compress_up_to - 1);
        }

        Some(compress_up_to - 1)
    }

    /// Build a prompt for compression
    fn build_compression_prompt(messages: &[crate::thread::Message]) -> String {
        let mut prompt = String::from(
            "Compress the following conversation into a concise summary that preserves all important context, \
             technical details, decisions made, and key information. Keep it under 500 tokens:\n\n",
        );

        for message in messages {
            let role_label = match message.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
                Role::System => "System",
            };

            prompt.push_str(&format!("{}:\n", role_label));

            // Add message content
            for segment in &message.segments {
                if let crate::thread::MessageSegment::Text(text) = segment {
                    prompt.push_str(text);
                    prompt.push('\n');
                }
            }

            // Add context if present
            if !message.loaded_context.text.is_empty() {
                prompt.push_str("Context:\n");
                prompt.push_str(&message.loaded_context.text);
                prompt.push('\n');
            }

            prompt.push('\n');
        }

        prompt
    }

    /// Mock compression for testing
    /// TODO: Replace with actual /completions API call
    fn mock_compress(content: &str) -> String {
        // Simulate compression by taking first 20% of content
        let compress_ratio = 0.2;
        let compressed_len = (content.len() as f64 * compress_ratio) as usize;

        format!(
            "[Compressed conversation summary]\n\
             This conversation covered the following key points:\n\
             - Initial setup and context establishment\n\
             - Technical implementation details discussed\n\
             - Decisions made and rationale\n\
             - Current state and next steps\n\n\
             [Details compressed from {} characters to {} characters]",
            content.len(),
            compressed_len
        )
    }

    /// Extract a brief summary from compressed content
    fn extract_summary(compressed: &str) -> String {
        // Take first line or first 100 chars as summary
        compressed
            .lines()
            .next()
            .unwrap_or("Compressed conversation")
            .chars()
            .take(100)
            .collect()
    }

    /// Call the compression service via the Go endpoint
    async fn call_compression_service(
        client: &Arc<Client>,
        messages: &[crate::thread::Message],
        model: &str,
        thread_id: &str,
    ) -> Result<CompressionResponse> {
        // Convert messages to API format
        let api_messages: Vec<ApiMessage> = messages
            .iter()
            .map(|msg| {
                let mut content = String::new();

                // Add loaded context if present
                if !msg.loaded_context.text.is_empty() {
                    content.push_str(&msg.loaded_context.text);
                    content.push_str("\n\n");
                }

                // Add message segments
                for segment in &msg.segments {
                    if let crate::thread::MessageSegment::Text(text) = segment {
                        content.push_str(text);
                    }
                }

                ApiMessage {
                    role: match msg.role {
                        Role::User => "user".to_string(),
                        Role::Assistant => "assistant".to_string(),
                        Role::System => "system".to_string(),
                    },
                    content,
                }
            })
            .collect();

        let request_body = CompressionRequest {
            messages: api_messages,
            model: model.to_string(),
            max_tokens: 4096, // Default max tokens for compression
            thread_id: Some(thread_id.to_string()),
            user_id: None,
            store_result: false,
        };

        // Acquire JWT token for authentication
        let llm_api_token = LlmApiToken::default();
        let mut token = llm_api_token.acquire(client).await?;
        let mut refreshed_token = false;

        let http_client = &client.http_client();

        loop {
            let request = http_client::Request::builder()
                .method(http_client::Method::POST)
                .uri(http_client.build_zed_llm_url("/compress", &[])?.as_ref())
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(serde_json::to_string(&request_body)?.into())?;

            let mut response = http_client.send(request).await?;

            if response.status().is_success() {
                let mut body = String::new();
                response.body_mut().read_to_string(&mut body).await?;

                // Log the raw compression response from server
                // log::info!("📊 /compress raw API Response: {}", body);
                let api_response: ApiCompressionResponse = serde_json::from_str(&body)?;

                return Ok(CompressionResponse {
                    compressed_content: api_response
                        .messages
                        .iter()
                        .map(|m| format!("{}: {}", m.role, m.content))
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                    summary: api_response
                        .summary
                        .unwrap_or_else(|| "Compressed conversation".to_string()),
                    compression_ratio: api_response.metrics.compression_ratio,
                    original_tokens: api_response.metrics.original_tokens,
                    compressed_tokens: api_response.metrics.compressed_tokens,
                });
            }

            // Check for expired token and retry once
            if !refreshed_token
                && response
                    .headers()
                    .get(EXPIRED_LLM_TOKEN_HEADER_NAME)
                    .is_some()
            {
                log::info!("Token expired for /compress, refreshing and retrying...");
                token = llm_api_token.refresh(client).await?;
                refreshed_token = true;
                continue;
            }

            // If not successful and not an expired token issue, return error
            anyhow::bail!("Compression service returned status: {}", response.status())
        }
    }
}

// Request/Response types for compression API
#[derive(Debug, Serialize)]
struct CompressionRequest {
    messages: Vec<ApiMessage>,
    model: String,
    max_tokens: usize,
    thread_id: Option<String>,
    user_id: Option<String>,
    store_result: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiCompressionResponse {
    messages: Vec<ApiMessage>,
    metrics: CompressionMetrics,
    summary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompressionMetrics {
    original_tokens: usize,
    compressed_tokens: usize,
    compression_ratio: f32,
}

struct CompressionResponse {
    compressed_content: String,
    summary: String,
    compression_ratio: f32,
    original_tokens: usize,
    compressed_tokens: usize,
}

/// Compression state tracking
#[derive(Debug, Clone)]
pub struct CompressionState {
    pub last_compression_at: Option<MessageId>,
    pub total_compressions: usize,
    pub total_tokens_saved: usize,
    pub is_compressing: bool,
}

impl Default for CompressionState {
    fn default() -> Self {
        Self {
            last_compression_at: None,
            total_compressions: 0,
            total_tokens_saved: 0,
            is_compressing: false,
        }
    }
}
