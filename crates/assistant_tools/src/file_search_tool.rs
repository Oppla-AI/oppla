use std::sync::Arc;

use crate::schema::json_schema_for;
use crate::ui::ToolCallCardHeader;
use agent_ui::IdeContext;
use anyhow::{Context as _, Result, anyhow};
use assistant_tool::{
    ActionLog, Tool, ToolCard, ToolResult, ToolResultContent, ToolResultOutput, ToolUseStatus,
};
use client::Client;
use futures::AsyncReadExt as _;
use gpui::{
    AnyWindowHandle, App, AppContext, Context, Entity, IntoElement, Task, WeakEntity, Window,
};
use http_client::{HttpClientWithUrl, Method};
use language_model::{LanguageModel, LanguageModelRequest, LanguageModelToolSchemaFormat, LlmApiToken};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ui::{Component, ComponentScope, Disclosure, IconName, Label, LabelSize, prelude::*};
use workspace::Workspace;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileSearchToolInput {
    /// The search query to find relevant context
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    
    /// Maximum number of results to return (default: 10, max: 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    
    /// Filter options for the search
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<SearchFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchFilter {
    /// Type of content to search: "conversations", "tasks", "compressed", or "all"
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    search_type: Option<String>,
    
    /// Content to extract: "work_item" (work item details only), "big_bet" (big bet details only), or "auto" (automatically decide based on context)
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    
    /// Optional thread ID to search within a specific thread
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_id: Option<String>,
    
    /// Optional account ID to filter results by account
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<String>,
    
    /// Optional product ID to filter results by product
    #[serde(skip_serializing_if = "Option::is_none")]
    product_id: Option<String>,
    
    /// Optional board ID to filter results by board
    #[serde(skip_serializing_if = "Option::is_none")]
    board_id: Option<String>,
    
    /// Optional task ID to filter results by specific task
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileSearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<SearchFilter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSearchResult {
    pub id: String,
    pub content: String,
    #[serde(rename = "type")]
    pub result_type: String,
    pub similarity: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSearchResponse {
    pub results: Vec<FileSearchResult>,
    pub total: usize,
    pub query: String,
}

pub struct FileSearchTool {
    http_client: Arc<HttpClientWithUrl>,
}

impl FileSearchTool {
    pub fn new(http_client: Arc<HttpClientWithUrl>) -> Self {
        Self { http_client }
    }

    async fn perform_search(
        http_client: Arc<HttpClientWithUrl>,
        input: FileSearchToolInput,
        llm_api_token: LlmApiToken,
        client: Arc<Client>,
        context_filters: Option<SearchFilter>,
    ) -> Result<FileSearchResponse> {
        // Acquire the token
        let token = llm_api_token.acquire(&client).await
            .context("Failed to acquire LLM API token")?;

        // Merge context filters with input filters
        let filter = if let Some(context_filter) = context_filters {
            let mut merged_filter = input.filter.unwrap_or_else(|| SearchFilter {
                search_type: None,
                content_type: None,
                thread_id: None,
                account_id: None,
                product_id: None,
                board_id: None,
                task_id: None,
            });
            
            // Only apply context filters if not already specified
            if merged_filter.account_id.is_none() {
                merged_filter.account_id = context_filter.account_id;
            }
            if merged_filter.product_id.is_none() {
                merged_filter.product_id = context_filter.product_id;
            }
            if merged_filter.board_id.is_none() {
                merged_filter.board_id = context_filter.board_id;
            }
            if merged_filter.task_id.is_none() {
                merged_filter.task_id = context_filter.task_id;
            }
            
            Some(merged_filter)
        } else {
            input.filter
        };

        // Build the request body
        let request_body = FileSearchRequest {
            query: input.query,
            limit: input.limit,
            filter,
        };

        // Build the URL for the search endpoint
        let url = http_client
            .build_zed_llm_url("/api/v1/search", &[])
            .context("Failed to build search URL")?;

        // Create the HTTP request
        let request = http_client::Request::builder()
            .method(Method::POST)
            .uri(url.as_ref())
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .body(serde_json::to_string(&request_body)?.into())?;

        // Send the request
        let mut response = http_client
            .send(request)
            .await
            .context("Failed to send search request")?;

        // Check response status
        if !response.status().is_success() {
            let mut body = String::new();
            response.body_mut().read_to_string(&mut body).await?;
            return Err(anyhow!(
                "Search request failed with status {}: {}",
                response.status(),
                body
            ));
        }

        // Read and parse the response
        let mut body = String::new();
        response.body_mut().read_to_string(&mut body).await?;
        let search_response: FileSearchResponse = serde_json::from_str(&body)
            .context("Failed to parse search response")?;

        Ok(search_response)
    }
}

impl Tool for FileSearchTool {
    fn name(&self) -> String {
        "file_search".into()
    }

    fn needs_confirmation(&self, _: &serde_json::Value, _: &App) -> bool {
        false
    }

    fn may_perform_edits(&self) -> bool {
        false
    }

    fn description(&self) -> String {
        "Search project planning context including big bet descriptions, work item details, requirements, and specifications. \
         Use this to understand what needs to be implemented and find acceptance criteria. \
         Filter by type: 'conversations', 'tasks' (work items), 'compressed', or 'all'. \
         Use content_type to get specific information: 'work_item' for work item details only, 'big_bet' for big bet overview only, or 'auto' (default) to automatically decide. \
         Automatically uses your synced big bet and work item context. Results include content, type, and similarity score."
            .into()
    }

    fn icon(&self) -> IconName {
        IconName::MagnifyingGlass
    }

    fn input_schema(&self, format: LanguageModelToolSchemaFormat) -> Result<serde_json::Value> {
        json_schema_for::<FileSearchToolInput>(format)
    }

    fn ui_text(&self, input: &serde_json::Value) -> String {
        match serde_json::from_value::<FileSearchToolInput>(input.clone()) {
            Ok(input) => {
                if let Some(query) = &input.query {
                    format!("Searching for \"{}\"", query)
                } else if let Some(filter) = &input.filter {
                    if let Some(thread_id) = &filter.thread_id {
                        format!("Searching thread {}", thread_id)
                    } else if let Some(search_type) = &filter.search_type {
                        format!("Searching {} content", search_type)
                    } else {
                        "Searching content".to_string()
                    }
                } else {
                    "Searching content".to_string()
                }
            }
            Err(_) => "Search content".to_string(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: serde_json::Value,
        _request: Arc<LanguageModelRequest>,
        _project: Entity<Project>,
        _action_log: Entity<ActionLog>,
        _model: Arc<dyn LanguageModel>,
        _window: Option<AnyWindowHandle>,
        cx: &mut App,
    ) -> ToolResult {
        let input = match serde_json::from_value::<FileSearchToolInput>(input) {
            Ok(input) => input,
            Err(err) => return Task::ready(Err(anyhow!(err))).into(),
        };

        // Validate input
        if input.query.is_none() && input.filter.as_ref().and_then(|f| f.thread_id.as_ref()).is_none() {
            return Task::ready(Err(anyhow!("Either 'query' or 'filter.thread_id' must be provided"))).into();
        }

        // Get the LLM API token and client
        let llm_api_token = LlmApiToken::default();
        let client = Client::global(cx);
        
        // Extract context filters from IdeContext if available
        let context_filters = cx.try_global::<IdeContext>()
            .and_then(|ide_context| ide_context.get_sync_data())
            .map(|sync_data| {
                // Only include filters that have values
                let mut filter = SearchFilter {
                    search_type: None,
                    content_type: Some("auto".to_string()), // Default to auto
                    thread_id: None,
                    account_id: None,
                    product_id: None,
                    board_id: None,
                    task_id: None,
                };
                
                // Always include account, product, and board if we have sync data
                filter.account_id = Some(sync_data.account_id.to_string());
                filter.product_id = Some(sync_data.product_id.to_string());
                filter.board_id = Some(sync_data.board_id.to_string());
                
                // Only include task_id if it's actually present
                filter.task_id = sync_data.task_id.map(|id| id.to_string());
                
                filter
            });
        
        let http_client = self.http_client.clone();
        let http_client2 = http_client.clone();
        let input2 = input.clone();
        let llm_api_token2 = llm_api_token.clone();
        let client2 = client.clone();
        let context_filters2 = context_filters.clone();
        
        let search_task = cx.background_spawn(async move {
            Self::perform_search(http_client, input, llm_api_token, client, context_filters).await
        });

        let card = cx.new(|cx| FileSearchToolCard::new(search_task, cx));

        let output = cx.background_spawn(async move {
            let response = Self::perform_search(http_client2, input2, llm_api_token2, client2, context_filters2).await?;
            
            let mut message = format!(
                "Found {} results",
                response.total
            );
            
            if !response.query.is_empty() {
                message.push_str(&format!(" for query \"{}\"", response.query));
            }
            
            if !response.results.is_empty() {
                message.push_str(":\n\n");
                for (i, result) in response.results.iter().enumerate() {
                    message.push_str(&format!(
                        "{}. [{}] (similarity: {:.2})\n{}\n\n",
                        i + 1,
                        result.result_type,
                        result.similarity,
                        if result.content.len() > 200 {
                            format!("{}...", &result.content[..200])
                        } else {
                            result.content.clone()
                        }
                    ));
                }
            }

            Ok(ToolResultOutput {
                content: ToolResultContent::Text(message),
                output: Some(serde_json::to_value(response)?),
            })
        });

        ToolResult {
            output,
            card: Some(card.into()),
        }
    }

    fn deserialize_card(
        self: Arc<Self>,
        output: serde_json::Value,
        _project: Entity<Project>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<assistant_tool::AnyToolCard> {
        let output = serde_json::from_value::<FileSearchResponse>(output).ok()?;
        let card = cx.new(|_| FileSearchToolCard::from_output(output));
        Some(card.into())
    }
}

#[derive(RegisterComponent)]
struct FileSearchToolCard {
    response: Option<Result<FileSearchResponse>>,
    expanded: bool,
    _task: Task<()>,
}

impl FileSearchToolCard {
    fn new(
        search_task: Task<Result<FileSearchResponse>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let _task = cx.spawn(async move |this, cx| {
            let response = search_task.await;
            this.update(cx, |this, cx| {
                this.response = Some(response);
                cx.notify();
            })
            .ok();
        });

        Self {
            response: None,
            expanded: false,
            _task,
        }
    }

    fn from_output(output: FileSearchResponse) -> Self {
        Self {
            response: Some(Ok(output)),
            expanded: false,
            _task: Task::ready(()),
        }
    }
}

impl ToolCard for FileSearchToolCard {
    fn render(
        &mut self,
        _status: &ToolUseStatus,
        _window: &mut Window,
        _workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let icon = IconName::MagnifyingGlass;

        let header = match self.response.as_ref() {
            Some(Ok(response)) => {
                let text: SharedString = if response.results.is_empty() {
                    "No results found".into()
                } else if response.results.len() == 1 {
                    "1 result".into()
                } else {
                    format!("{} results", response.results.len()).into()
                };
                ToolCallCardHeader::new(icon, "Searched Content").with_secondary_text(text)
            }
            Some(Err(error)) => {
                ToolCallCardHeader::new(icon, "Content Search").with_error(error.to_string())
            }
            None => ToolCallCardHeader::new(icon, "Searching Content").loading(),
        };

        let content = if self.expanded {
            self.response.as_ref().and_then(|response| match response {
                Ok(response) if !response.results.is_empty() => Some(
                    v_flex()
                        .overflow_hidden()
                        .ml_1p5()
                        .pl(px(5.))
                        .border_l_1()
                        .border_color(cx.theme().colors().border_variant)
                        .gap_2()
                        .children(response.results.iter().enumerate().map(|(_index, result)| {
                            v_flex()
                                .gap_1()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .px_1()
                                                .rounded_md()
                                                .bg(cx.theme().colors().element_background)
                                                .child(
                                                    Label::new(format!("[{}]", result.result_type))
                                                        .size(LabelSize::Small)
                                                        .color(Color::Muted),
                                                ),
                                        )
                                        .child(
                                            Label::new(format!("Similarity: {:.2}", result.similarity))
                                                .size(LabelSize::Small)
                                                .color(Color::Muted),
                                        ),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .bg(cx.theme().colors().element_background)
                                        .child(
                                            Label::new(if result.content.len() > 300 {
                                                format!("{}...", &result.content[..300])
                                            } else {
                                                result.content.clone()
                                            })
                                            .size(LabelSize::Small)
                                            .color(Color::Default)
                                        )
                                )
                        }))
                        .into_any(),
                ),
                _ => None,
            })
        } else {
            None
        };

        v_flex()
            .mb_3()
            .gap_1()
            .child(
                header.disclosure_slot(
                    Disclosure::new("file-search-disclosure", self.expanded)
                        .opened_icon(IconName::ChevronUp)
                        .closed_icon(IconName::ChevronDown)
                        .disabled(self.response.as_ref().map_or(true, |r| {
                            r.as_ref().map_or(true, |res| res.results.is_empty())
                        }))
                        .on_click(cx.listener(move |this, _, _, _cx| {
                            this.expanded = !this.expanded;
                        })),
                ),
            )
            .children(content)
    }
}

impl Component for FileSearchToolCard {
    fn scope() -> ComponentScope {
        ComponentScope::Agent
    }

    fn preview(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let successful_card = cx.new(|_| FileSearchToolCard {
            response: Some(Ok(FileSearchResponse {
                results: vec![
                    FileSearchResult {
                        id: "1".to_string(),
                        content: "User mentioned they want to implement vim mode with yank functionality".to_string(),
                        result_type: "conversation".to_string(),
                        similarity: 0.92,
                        metadata: serde_json::json!({}),
                    },
                    FileSearchResult {
                        id: "2".to_string(),
                        content: "Task: Implement yank mode for vim - Status: In Progress".to_string(),
                        result_type: "task".to_string(),
                        similarity: 0.87,
                        metadata: serde_json::json!({}),
                    },
                ],
                total: 2,
                query: "vim yank mode".to_string(),
            })),
            expanded: true,
            _task: Task::ready(()),
        });

        let empty_card = cx.new(|_| FileSearchToolCard {
            response: Some(Ok(FileSearchResponse {
                results: Vec::new(),
                total: 0,
                query: "nonexistent query".to_string(),
            })),
            expanded: false,
            _task: Task::ready(()),
        });

        Some(
            v_flex()
                .gap_6()
                .children(vec![ui::example_group(vec![
                    ui::single_example(
                        "With Results",
                        div()
                            .size_full()
                            .child(successful_card.update(cx, |tool, cx| {
                                tool.render(
                                    &ToolUseStatus::Finished("".into()),
                                    window,
                                    WeakEntity::new_invalid(),
                                    cx,
                                )
                                .into_any_element()
                            }))
                            .into_any_element(),
                    ),
                    ui::single_example(
                        "No Results",
                        div()
                            .size_full()
                            .child(empty_card.update(cx, |tool, cx| {
                                tool.render(
                                    &ToolUseStatus::Finished("".into()),
                                    window,
                                    WeakEntity::new_invalid(),
                                    cx,
                                )
                                .into_any_element()
                            }))
                            .into_any_element(),
                    ),
                ])])
                .into_any_element(),
        )
    }
}