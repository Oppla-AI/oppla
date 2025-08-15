mod add_llm_provider_modal;
mod configure_context_server_modal;
mod manage_profiles_modal;
mod tool_picker;

use std::{sync::Arc, time::Duration};

use agent_settings::AgentSettings;
use anyhow::Context as _;
use assistant_tool::{ToolSource, ToolWorkingSet};
use client::Client;
use collections::HashMap;
use context_server::ContextServerId;
use extension::ExtensionManifest;
use extension_host::ExtensionStore;
use fs::Fs;
use gpui::{
    Action, Animation, AnimationExt as _, AnyView, App, Corner, Entity, EventEmitter, FocusHandle,
    Focusable, ScrollHandle, Subscription, Task, Transformation, WeakEntity, percentage,
};
use language::LanguageRegistry;
use language_model::{
    LanguageModelProvider, LanguageModelProviderId, LanguageModelRegistry, ZED_CLOUD_PROVIDER_ID,
};
use notifications::status_toast::{StatusToast, ToastIcon};
use oppla_actions::ExtensionCategoryFilter;
use project::{
    context_server_store::{ContextServerConfiguration, ContextServerStatus, ContextServerStore},
    project_settings::{ContextServerSettings, ProjectSettings},
};
use proto::{self, Plan};
use settings::{Settings, update_settings_file};
use ui::{
    Chip, ContextMenu, Disclosure, Divider, DividerColor, ElevationIndex, Indicator, PopoverMenu,
    Scrollbar, ScrollbarState, Switch, SwitchColor, SwitchField, Tooltip, prelude::*,
};
use url::Url;
use util::ResultExt as _;
use workspace::Workspace;

pub(crate) use configure_context_server_modal::ConfigureContextServerModal;
pub(crate) use manage_profiles_modal::ManageProfilesModal;

// Global IDE context for storing synced task information
use gpui::Global;
use serde_json;
use std::sync::RwLock;

pub struct IdeContext {
    pub sync_data: RwLock<Option<TaskSyncData>>,
}

impl Global for IdeContext {}

impl IdeContext {
    pub fn init(cx: &mut App) {
        cx.set_global(IdeContext {
            sync_data: RwLock::new(None),
        });
    }

    pub fn get_sync_data(&self) -> Option<TaskSyncData> {
        self.sync_data.read().ok()?.clone()
    }

    pub fn set_sync_data(&self, data: TaskSyncData) {
        if let Ok(mut sync_data) = self.sync_data.write() {
            *sync_data = Some(data);
        }
    }

    pub fn clear_sync_data(&self) {
        if let Ok(mut sync_data) = self.sync_data.write() {
            *sync_data = None;
        }
    }

    // Helper method to get context filter for API searches
    pub fn get_context_filter(&self) -> Option<serde_json::Value> {
        let data = self.get_sync_data()?;

        let mut filter = serde_json::json!({
            "type": "tasks"
        });

        if let Some(filter_obj) = filter.as_object_mut() {
            filter_obj.insert(
                "account_id".to_string(),
                serde_json::Value::String(data.account_id.to_string()),
            );
            filter_obj.insert(
                "product_id".to_string(),
                serde_json::Value::String(data.product_id.to_string()),
            );
            filter_obj.insert(
                "board_id".to_string(),
                serde_json::Value::String(data.board_id.to_string()),
            );

            if let Some(task_id) = data.task_id {
                filter_obj.insert(
                    "task_id".to_string(),
                    serde_json::Value::String(task_id.to_string()),
                );
            }
        }

        Some(filter)
    }
}

use crate::{
    AddContextServer,
    agent_configuration::add_llm_provider_modal::{AddLlmProviderModal, LlmCompatibleProvider},
};

#[derive(Clone, Debug)]
pub struct TaskSyncData {
    // Account information
    pub account_id: SharedString,
    pub account_name: SharedString,

    // Product information
    pub product_id: SharedString,
    pub product_name: SharedString,

    // Big Bet (Board) information
    pub board_id: SharedString, // Store the board ID for API searches
    pub big_bet: Option<SharedString>, // Display name for Big Bet (board name)
    pub big_bet_description: Option<SharedString>, // Board description

    // Work Item (Task) information (optional)
    pub task_id: Option<SharedString>, // Store the task ID for API searches
    pub work_item: Option<SharedString>, // Display name for Work Item (task name)
    pub work_item_description: Option<SharedString>, // Task description

    // Metadata
    pub synced_at: Option<std::time::SystemTime>,
}

pub struct AgentConfiguration {
    fs: Arc<dyn Fs>,
    language_registry: Arc<LanguageRegistry>,
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    configuration_views_by_provider: HashMap<LanguageModelProviderId, AnyView>,
    context_server_store: Entity<ContextServerStore>,
    expanded_context_server_tools: HashMap<ContextServerId, bool>,
    expanded_provider_configurations: HashMap<LanguageModelProviderId, bool>,
    tools: Entity<ToolWorkingSet>,
    _registry_subscription: Subscription,
    scroll_handle: ScrollHandle,
    scrollbar_state: ScrollbarState,
    task_sync_expanded: bool,
    task_sync_data: Option<TaskSyncData>,
}

impl AgentConfiguration {
    pub fn new(
        fs: Arc<dyn Fs>,
        context_server_store: Entity<ContextServerStore>,
        tools: Entity<ToolWorkingSet>,
        language_registry: Arc<LanguageRegistry>,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        let registry_subscription = cx.subscribe_in(
            &LanguageModelRegistry::global(cx),
            window,
            |this, _, event: &language_model::Event, window, cx| match event {
                language_model::Event::AddedProvider(provider_id) => {
                    let provider = LanguageModelRegistry::read_global(cx).provider(provider_id);
                    if let Some(provider) = provider {
                        this.add_provider_configuration_view(&provider, window, cx);
                    }
                }
                language_model::Event::RemovedProvider(provider_id) => {
                    this.remove_provider_configuration_view(provider_id);
                }
                _ => {}
            },
        );

        cx.subscribe(&context_server_store, |_, _, _, cx| cx.notify())
            .detach();

        let scroll_handle = ScrollHandle::new();
        let scrollbar_state = ScrollbarState::new(scroll_handle.clone());

        let mut expanded_provider_configurations = HashMap::default();
        if LanguageModelRegistry::read_global(cx)
            .provider(&ZED_CLOUD_PROVIDER_ID)
            .map_or(false, |cloud_provider| cloud_provider.must_accept_terms(cx))
        {
            expanded_provider_configurations.insert(ZED_CLOUD_PROVIDER_ID, true);
        }

        let mut this = Self {
            fs,
            language_registry,
            workspace,
            focus_handle,
            configuration_views_by_provider: HashMap::default(),
            context_server_store,
            expanded_context_server_tools: HashMap::default(),
            expanded_provider_configurations,
            tools,
            _registry_subscription: registry_subscription,
            scroll_handle,
            scrollbar_state,
            task_sync_expanded: true, // Start expanded if no task is synced
            task_sync_data: None,     // Initially no task is synced
        };
        this.build_provider_configuration_views(window, cx);
        this
    }

    fn build_provider_configuration_views(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let providers = LanguageModelRegistry::read_global(cx).providers();
        for provider in providers {
            self.add_provider_configuration_view(&provider, window, cx);
        }
    }

    fn remove_provider_configuration_view(&mut self, provider_id: &LanguageModelProviderId) {
        self.configuration_views_by_provider.remove(provider_id);
        self.expanded_provider_configurations.remove(provider_id);
    }

    fn add_provider_configuration_view(
        &mut self,
        provider: &Arc<dyn LanguageModelProvider>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let configuration_view = provider.configuration_view(window, cx);
        self.configuration_views_by_provider
            .insert(provider.id(), configuration_view);
    }
}

impl Focusable for AgentConfiguration {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub enum AssistantConfigurationEvent {
    NewThread(Arc<dyn LanguageModelProvider>),
}

impl EventEmitter<AssistantConfigurationEvent> for AgentConfiguration {}

impl AgentConfiguration {
    fn render_provider_configuration_block(
        &mut self,
        provider: &Arc<dyn LanguageModelProvider>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let provider_id = provider.id().0.clone();
        let provider_name = provider.name().0.clone();
        let provider_id_string = SharedString::from(format!("provider-disclosure-{provider_id}"));

        let configuration_view = self
            .configuration_views_by_provider
            .get(&provider.id())
            .cloned();

        let is_expanded = self
            .expanded_provider_configurations
            .get(&provider.id())
            .copied()
            .unwrap_or(false);

        let is_zed_provider = provider.id() == ZED_CLOUD_PROVIDER_ID;
        let current_plan = if is_zed_provider {
            self.workspace
                .upgrade()
                .and_then(|workspace| workspace.read(cx).user_store().read(cx).current_plan())
        } else {
            None
        };

        let is_signed_in = self
            .workspace
            .read_with(cx, |workspace, _| {
                workspace.client().status().borrow().is_connected()
            })
            .unwrap_or(false);

        v_flex()
            .w_full()
            .when(is_expanded, |this| this.mb_2())
            .child(
                div()
                    .opacity(0.6)
                    .px_2()
                    .child(Divider::horizontal().color(DividerColor::Border)),
            )
            .child(
                h_flex()
                    .map(|this| {
                        if is_expanded {
                            this.mt_2().mb_1()
                        } else {
                            this.my_2()
                        }
                    })
                    .w_full()
                    .justify_between()
                    .child(
                        h_flex()
                            .id(provider_id_string.clone())
                            .cursor_pointer()
                            .px_2()
                            .py_0p5()
                            .w_full()
                            .justify_between()
                            .rounded_sm()
                            .hover(|hover| hover.bg(cx.theme().colors().element_hover))
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(
                                        Icon::new(provider.icon())
                                            .size(IconSize::Small)
                                            .color(Color::Muted),
                                    )
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .gap_1()
                                            .child(
                                                Label::new(provider_name.clone())
                                                    .size(LabelSize::Large),
                                            )
                                            .map(|this| {
                                                if is_zed_provider && is_signed_in {
                                                    this.child(
                                                        self.render_zed_plan_info(current_plan, cx),
                                                    )
                                                } else {
                                                    this.when(
                                                        provider.is_authenticated(cx)
                                                            && !is_expanded,
                                                        |parent| {
                                                            parent.child(
                                                                Icon::new(IconName::Check)
                                                                    .color(Color::Success),
                                                            )
                                                        },
                                                    )
                                                }
                                            }),
                                    ),
                            )
                            .child(
                                Disclosure::new(provider_id_string, is_expanded)
                                    .opened_icon(IconName::ChevronUp)
                                    .closed_icon(IconName::ChevronDown),
                            )
                            .on_click(cx.listener({
                                let provider_id = provider.id().clone();
                                move |this, _event, _window, _cx| {
                                    let is_expanded = this
                                        .expanded_provider_configurations
                                        .entry(provider_id.clone())
                                        .or_insert(false);

                                    *is_expanded = !*is_expanded;
                                }
                            })),
                    )
                    .when(provider.is_authenticated(cx), |parent| {
                        parent.child(
                            Button::new(
                                SharedString::from(format!("new-thread-{provider_id}")),
                                "Start New Thread",
                            )
                            .icon_position(IconPosition::Start)
                            .icon(IconName::Plus)
                            .icon_size(IconSize::Small)
                            .icon_color(Color::Muted)
                            .label_size(LabelSize::Small)
                            .on_click(cx.listener({
                                let provider = provider.clone();
                                move |_this, _event, _window, cx| {
                                    cx.emit(AssistantConfigurationEvent::NewThread(
                                        provider.clone(),
                                    ))
                                }
                            })),
                        )
                    }),
            )
            .child(
                div()
                    .px_2()
                    .when(is_expanded, |parent| match configuration_view {
                        Some(configuration_view) => parent.child(configuration_view),
                        None => parent.child(Label::new(format!(
                            "No configuration view for {provider_name}",
                        ))),
                    }),
            )
    }

    fn render_provider_configuration_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let providers = LanguageModelRegistry::read_global(cx).providers();

        v_flex()
            .w_full()
            .child(
                h_flex()
                    .p(DynamicSpacing::Base16.rems(cx))
                    .pr(DynamicSpacing::Base20.rems(cx))
                    .pb_0()
                    .mb_2p5()
                    .items_start()
                    .justify_between()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_0p5()
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .justify_between()
                                    .child(Headline::new("LLM Providers"))
                                    .child(
                                        PopoverMenu::new("add-provider-popover")
                                            .trigger(
                                                Button::new("add-provider", "Add Provider")
                                                    .icon_position(IconPosition::Start)
                                                    .icon(IconName::Plus)
                                                    .icon_size(IconSize::Small)
                                                    .icon_color(Color::Muted)
                                                    .label_size(LabelSize::Small),
                                            )
                                            .anchor(gpui::Corner::TopRight)
                                            .menu({
                                                let workspace = self.workspace.clone();
                                                move |window, cx| {
                                                    Some(ContextMenu::build(
                                                        window,
                                                        cx,
                                                        |menu, _window, _cx| {
                                                            menu.header("Compatible APIs").entry(
                                                                "OpenAI",
                                                                None,
                                                                {
                                                                    let workspace =
                                                                        workspace.clone();
                                                                    move |window, cx| {
                                                                        workspace
                                                        .update(cx, |workspace, cx| {
                                                            AddLlmProviderModal::toggle(
                                                                LlmCompatibleProvider::OpenAi,
                                                                workspace,
                                                                window,
                                                                cx,
                                                            );
                                                        })
                                                        .log_err();
                                                                    }
                                                                },
                                                            )
                                                        },
                                                    ))
                                                }
                                            }),
                                    ),
                            )
                            .child(
                                Label::new("Add at least one provider to use AI-powered features.")
                                    .color(Color::Muted),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .pl(DynamicSpacing::Base08.rems(cx))
                    .pr(DynamicSpacing::Base20.rems(cx))
                    .children(
                        providers.into_iter().map(|provider| {
                            self.render_provider_configuration_block(&provider, cx)
                        }),
                    ),
            )
    }

    fn render_command_permission(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let always_allow_tool_actions = AgentSettings::get_global(cx).always_allow_tool_actions;
        let fs = self.fs.clone();

        SwitchField::new(
            "always-allow-tool-actions-switch",
            "Allow running commands without asking for confirmation",
            "The agent can perform potentially destructive actions without asking for your confirmation.",
            always_allow_tool_actions,
            move |state, _window, cx| {
                let allow = state == &ToggleState::Selected;
                update_settings_file::<AgentSettings>(fs.clone(), cx, move |settings, _| {
                    settings.set_always_allow_tool_actions(allow);
                });
            },
        )
    }

    fn render_single_file_review(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let single_file_review = AgentSettings::get_global(cx).single_file_review;
        let fs = self.fs.clone();

        SwitchField::new(
            "single-file-review",
            "Enable single-file agent reviews",
            "Agent edits are also displayed in single-file editors for review.",
            single_file_review,
            move |state, _window, cx| {
                let allow = state == &ToggleState::Selected;
                update_settings_file::<AgentSettings>(fs.clone(), cx, move |settings, _| {
                    settings.set_single_file_review(allow);
                });
            },
        )
    }

    fn render_sound_notification(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let play_sound_when_agent_done = AgentSettings::get_global(cx).play_sound_when_agent_done;
        let fs = self.fs.clone();

        SwitchField::new(
            "sound-notification",
            "Play sound when finished generating",
            "Hear a notification sound when the agent is done generating changes or needs your input.",
            play_sound_when_agent_done,
            move |state, _window, cx| {
                let allow = state == &ToggleState::Selected;
                update_settings_file::<AgentSettings>(fs.clone(), cx, move |settings, _| {
                    settings.set_play_sound_when_agent_done(allow);
                });
            },
        )
    }

    fn render_modifier_to_send(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let use_modifier_to_send = AgentSettings::get_global(cx).use_modifier_to_send;
        let fs = self.fs.clone();

        SwitchField::new(
            "modifier-send",
            "Use modifier to submit a message",
            "Make a modifier (cmd-enter on macOS, ctrl-enter on Linux) required to send messages.",
            use_modifier_to_send,
            move |state, _window, cx| {
                let allow = state == &ToggleState::Selected;
                update_settings_file::<AgentSettings>(fs.clone(), cx, move |settings, _| {
                    settings.set_use_modifier_to_send(allow);
                });
            },
        )
    }

    fn render_general_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p(DynamicSpacing::Base16.rems(cx))
            .pr(DynamicSpacing::Base20.rems(cx))
            .gap_2p5()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(Headline::new("General Settings"))
            .child(self.render_command_permission(cx))
            .child(self.render_single_file_review(cx))
            .child(self.render_sound_notification(cx))
            .child(self.render_modifier_to_send(cx))
    }

    fn sync_task(&mut self, cx: &mut Context<Self>) {
        // Get the client to acquire JWT token
        let client = Client::global(cx).clone();
        let workspace = self.workspace.clone();

        // Spawn an async task to get the token and handle the sync flow
        cx.spawn(async move |this, cx| {
            let background = cx.background_executor().clone();

            // Try to acquire the LLM token
            let token_result = client.request(proto::GetLlmToken {}).await;

            match token_result {
                Ok(response) => {
                    let token = response.token;

                    // Start a local HTTP server to receive the callback
                    let server = tiny_http::Server::http("127.0.0.1:0")
                        .expect("failed to find open port for sync callback");
                    let port = server.server_addr().port();

                    // Build the URL with token and callback port
                    let url = format!(
                        "https://app.oppla.ai/home/ide?token={}&callback_port={}",
                        token, port
                    );

                    // Open the URL in the default browser
                    cx.update(|cx| {
                        cx.open_url(&url);
                    }).log_err();

                    // Listen for the callback with sync data
                    let sync_result = background.spawn(async move {
                        for _ in 0..300 { // Wait up to 5 minutes (300 seconds)
                            if let Some(req) = server.recv_timeout(std::time::Duration::from_secs(1)).ok().flatten() {
                                let path = req.url();
                                let url = Url::parse(&format!("http://example.com{}", path))
                                    .context("failed to parse sync callback url")?;

                                // Parse the sync data from query parameters
                                let mut sync_data = TaskSyncData {
                                    account_id: SharedString::default(),
                                    account_name: SharedString::default(),
                                    product_id: SharedString::default(),
                                    product_name: SharedString::default(),
                                    board_id: SharedString::default(),
                                    big_bet: None,
                                    big_bet_description: None,
                                    task_id: None,
                                    work_item: None,
                                    work_item_description: None,
                                    synced_at: Some(std::time::SystemTime::now()),
                                };

                                for (key, value) in url.query_pairs() {
                                    match key.as_ref() {
                                        "account_id" => sync_data.account_id = SharedString::from(value.to_string()),
                                        "account_name" => sync_data.account_name = SharedString::from(value.to_string()),
                                        "product_id" => sync_data.product_id = SharedString::from(value.to_string()),
                                        "product_name" => sync_data.product_name = SharedString::from(value.to_string()),
                                        "board_id" => sync_data.board_id = SharedString::from(value.to_string()),
                                        "board_name" => sync_data.big_bet = Some(SharedString::from(value.to_string())),
                                        "board_description" => sync_data.big_bet_description = Some(SharedString::from(value.to_string())),
                                        "task_id" => sync_data.task_id = Some(SharedString::from(value.to_string())),
                                        "task_name" => sync_data.work_item = Some(SharedString::from(value.to_string())),
                                        "task_description" => sync_data.work_item_description = Some(SharedString::from(value.to_string())),
                                        _ => {}
                                    }
                                }

                                // Send success response and redirect to close the tab
                                let response_html = r#"<!DOCTYPE html>
                                <html>
                                <head>
                                    <title>Sync Complete</title>
                                    <script>window.close();</script>
                                </head>
                                <body>
                                    <h1>Sync Complete!</h1>
                                    <p>You can close this tab and return to Oppla IDE.</p>
                                </body>
                                </html>"#;

                                req.respond(
                                    tiny_http::Response::from_string(response_html)
                                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
                                ).context("failed to respond to sync callback")?;

                                return Ok(sync_data);
                            }
                        }
                        anyhow::bail!("Sync timeout - no callback received")
                    }).await;

                    // Update the sync data if successful
                    if let Ok(sync_data) = sync_result {
                        cx.update(|cx| {
                            if let Some(this) = this.upgrade() {
                                this.update(cx, |this, cx| {
                                    this.update_sync_data(sync_data, cx);
                                });
                            }
                        }).log_err();
                    }
                },
                Err(err) => {
                    log::error!("Failed to acquire JWT token for task sync: {}", err);

                    // Show user-friendly error message
                    cx.update(|cx| {
                        workspace
                            .update(cx, |workspace, cx| {
                                workspace.toggle_status_toast(
                                    StatusToast::new(
                                        "Unable to sync task. Please ensure you're signed in to Oppla and try again.",
                                        cx,
                                        |this, _cx| {
                                            this.icon(ToastIcon::new(IconName::XCircle).color(Color::Error))
                                                .dismiss_button(true)
                                                .action("Sign In", move |_, cx| {
                                                    cx.open_url("https://app.oppla.ai/auth/sign-in");
                                                })
                                        },
                                    ),
                                    cx,
                                );
                            })
                            .log_err();
                    }).log_err();
                }
            }

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn sync_latest_task(&mut self, cx: &mut Context<Self>) {
        // Use the same sync flow as sync_task to open the sync page
        self.sync_task(cx);
    }

    fn clear_task_sync(&mut self, cx: &mut Context<Self>) {
        // Clear the synced task data
        self.task_sync_data = None;
        // Clear global context as well
        if let Some(ide_context) = cx.try_global::<IdeContext>() {
            ide_context.clear_sync_data();
        }
        // Expand the section when cleared so user can sync again
        self.task_sync_expanded = true;
        cx.notify();
    }

    // Method to update sync data after successful sync from web app
    pub fn update_sync_data(&mut self, data: TaskSyncData, cx: &mut Context<Self>) {
        self.task_sync_data = Some(data.clone());

        // Store in global context for access across the IDE
        if let Some(ide_context) = cx.try_global::<IdeContext>() {
            ide_context.set_sync_data(data);
        } else {
            // Initialize global context if not already done
            IdeContext::init(cx);
            if let Some(ide_context) = cx.try_global::<IdeContext>() {
                ide_context.set_sync_data(data);
            }
        }

        // Collapse the section after syncing
        self.task_sync_expanded = false;
        cx.notify();
    }

    fn render_task_sync_section(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_expanded = self.task_sync_expanded;

        v_flex()
            .p(DynamicSpacing::Base16.rems(cx))
            .pr(DynamicSpacing::Base20.rems(cx))
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Headline::new("Task Context Sync"))
                            )
                            .child(
                                Label::new("Sync your current task to help the AI understand what you're working on")
                                    .color(Color::Muted)
                            )
                    )
                    .child(
                        Disclosure::new("task-sync-disclosure", is_expanded)
                            .opened_icon(IconName::ChevronUp)
                            .closed_icon(IconName::ChevronDown)
                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                this.task_sync_expanded = !this.task_sync_expanded;
                                _cx.notify();
                            }))
                    )
            )
            .when(is_expanded, |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .mt_2()
                        .when_some(self.task_sync_data.clone(), |this, task_data| {
                            this.child(
                                v_flex()
                                    .gap_1()
                                    .p_2()
                                    .bg(cx.theme().colors().element_background)
                                    .rounded_md()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(Label::new("Product:").color(Color::Muted))
                                            .child(Label::new(task_data.product_name))
                                    )
                                    .when_some(task_data.big_bet, |this, big_bet| {
                                        this.child(
                                            h_flex()
                                                .gap_2()
                                                .child(Label::new("Big Bet:").color(Color::Muted))
                                                .child(Label::new(big_bet))
                                        )
                                    })
                                    .when_some(task_data.work_item, |this, work_item| {
                                        this.child(
                                            h_flex()
                                                .gap_2()
                                                .child(Label::new("Work Item:").color(Color::Muted))
                                                .child(Label::new(work_item))
                                        )
                                    })
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("sync-latest", "Sync Latest Information")
                                            .style(ButtonStyle::Filled)
                                            .icon(IconName::ArrowCircle)
                                            .icon_position(IconPosition::Start)
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                // Placeholder: This will sync the latest task information
                                                this.sync_latest_task(cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("clear-sync", "Clear Sync")
                                            .style(ButtonStyle::Subtle)
                                            .icon(IconName::Trash)
                                            .icon_position(IconPosition::Start)
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.clear_task_sync(cx);
                                            }))
                                    )
                            )
                        })
                        .when(self.task_sync_data.is_none(), |this| {
                            this.child(
                                Button::new("sync-task", "Sync Your Task")
                                    .style(ButtonStyle::Filled)
                                    .layer(ElevationIndex::ModalSurface)
                                    .full_width()
                                    .icon(IconName::ArrowCircle)
                                    .icon_size(IconSize::Small)
                                    .icon_position(IconPosition::Start)
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.sync_task(cx);
                                    }))
                            )
                        })
                )
            })
    }

    fn render_zed_plan_info(&self, plan: Option<Plan>, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(plan) = plan {
            let free_chip_bg = cx
                .theme()
                .colors()
                .editor_background
                .opacity(0.5)
                .blend(cx.theme().colors().text_accent.opacity(0.05));

            let pro_chip_bg = cx
                .theme()
                .colors()
                .editor_background
                .opacity(0.5)
                .blend(cx.theme().colors().text_accent.opacity(0.2));

            let (plan_name, label_color, bg_color) = match plan {
                Plan::Free => ("Free", Color::Default, free_chip_bg),
                Plan::ZedProTrial => ("Pro Trial", Color::Accent, pro_chip_bg),
                Plan::ZedPro => ("Pro", Color::Accent, pro_chip_bg),
            };

            Chip::new(plan_name.to_string())
                .bg_color(bg_color)
                .label_color(label_color)
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    fn render_context_servers_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let context_server_ids = self.context_server_store.read(cx).configured_server_ids();

        v_flex()
            .p(DynamicSpacing::Base16.rems(cx))
            .pr(DynamicSpacing::Base20.rems(cx))
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                v_flex()
                    .gap_0p5()
                    .child(Headline::new("Model Context Protocol (MCP) Servers"))
                    .child(Label::new("Connect to context servers via the Model Context Protocol either via Oppla extensions or directly.").color(Color::Muted)),
            )
            .children(
                context_server_ids.into_iter().map(|context_server_id| {
                    self.render_context_server(context_server_id, window, cx)
                }),
            )
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex().w_full().child(
                            Button::new("add-context-server", "Add Custom Server")
                                .style(ButtonStyle::Filled)
                                .layer(ElevationIndex::ModalSurface)
                                .full_width()
                                .icon(IconName::Plus)
                                .icon_size(IconSize::Small)
                                .icon_position(IconPosition::Start)
                                .on_click(|_event, window, cx| {
                                    window.dispatch_action(AddContextServer.boxed_clone(), cx)
                                }),
                        ),
                    )
                    .child(
                        h_flex().w_full().child(
                            Button::new(
                                "install-context-server-extensions",
                                "Install MCP Extensions",
                            )
                            .style(ButtonStyle::Filled)
                            .layer(ElevationIndex::ModalSurface)
                            .full_width()
                            .icon(IconName::Hammer)
                            .icon_size(IconSize::Small)
                            .icon_position(IconPosition::Start)
                            .on_click(|_event, window, cx| {
                                window.dispatch_action(
                                    oppla_actions::Extensions {
                                        category_filter: Some(
                                            ExtensionCategoryFilter::ContextServers,
                                        ),
                                        id: None,
                                    }
                                    .boxed_clone(),
                                    cx,
                                )
                            }),
                        ),
                    ),
            )
    }

    fn render_context_server(
        &self,
        context_server_id: ContextServerId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl use<> + IntoElement {
        let tools_by_source = self.tools.read(cx).tools_by_source(cx);
        let server_status = self
            .context_server_store
            .read(cx)
            .status_for_server(&context_server_id)
            .unwrap_or(ContextServerStatus::Stopped);
        let server_configuration = self
            .context_server_store
            .read(cx)
            .configuration_for_server(&context_server_id);

        let is_running = matches!(server_status, ContextServerStatus::Running);
        let item_id = SharedString::from(context_server_id.0.clone());
        let is_from_extension = server_configuration
            .as_ref()
            .map(|config| {
                matches!(
                    config.as_ref(),
                    ContextServerConfiguration::Extension { .. }
                )
            })
            .unwrap_or(false);

        let error = if let ContextServerStatus::Error(error) = server_status.clone() {
            Some(error)
        } else {
            None
        };

        let are_tools_expanded = self
            .expanded_context_server_tools
            .get(&context_server_id)
            .copied()
            .unwrap_or_default();
        let tools = tools_by_source
            .get(&ToolSource::ContextServer {
                id: context_server_id.0.clone().into(),
            })
            .map_or([].as_slice(), |tools| tools.as_slice());
        let tool_count = tools.len();

        let border_color = cx.theme().colors().border.opacity(0.6);

        let (source_icon, source_tooltip) = if is_from_extension {
            (
                IconName::ZedMcpExtension,
                "This MCP server was installed from an extension.",
            )
        } else {
            (
                IconName::ZedMcpCustom,
                "This custom MCP server was installed directly.",
            )
        };

        let (status_indicator, tooltip_text) = match server_status {
            ContextServerStatus::Starting => (
                Icon::new(IconName::LoadCircle)
                    .size(IconSize::XSmall)
                    .color(Color::Accent)
                    .with_animation(
                        SharedString::from(format!("{}-starting", context_server_id.0.clone(),)),
                        Animation::new(Duration::from_secs(3)).repeat(),
                        |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                    )
                    .into_any_element(),
                "Server is starting.",
            ),
            ContextServerStatus::Running => (
                Indicator::dot().color(Color::Success).into_any_element(),
                "Server is active.",
            ),
            ContextServerStatus::Error(_) => (
                Indicator::dot().color(Color::Error).into_any_element(),
                "Server has an error.",
            ),
            ContextServerStatus::Stopped => (
                Indicator::dot().color(Color::Muted).into_any_element(),
                "Server is stopped.",
            ),
        };

        let context_server_configuration_menu = PopoverMenu::new("context-server-config-menu")
            .trigger_with_tooltip(
                IconButton::new("context-server-config-menu", IconName::Settings)
                    .icon_color(Color::Muted)
                    .icon_size(IconSize::Small),
                Tooltip::text("Open MCP server options"),
            )
            .anchor(Corner::TopRight)
            .menu({
                let fs = self.fs.clone();
                let context_server_id = context_server_id.clone();
                let language_registry = self.language_registry.clone();
                let context_server_store = self.context_server_store.clone();
                let workspace = self.workspace.clone();
                move |window, cx| {
                    Some(ContextMenu::build(window, cx, |menu, _window, _cx| {
                        menu.entry("Configure Server", None, {
                            let context_server_id = context_server_id.clone();
                            let language_registry = language_registry.clone();
                            let workspace = workspace.clone();
                            move |window, cx| {
                                ConfigureContextServerModal::show_modal_for_existing_server(
                                    context_server_id.clone(),
                                    language_registry.clone(),
                                    workspace.clone(),
                                    window,
                                    cx,
                                )
                                .detach_and_log_err(cx);
                            }
                        })
                        .separator()
                        .entry("Uninstall", None, {
                            let fs = fs.clone();
                            let context_server_id = context_server_id.clone();
                            let context_server_store = context_server_store.clone();
                            let workspace = workspace.clone();
                            move |_, cx| {
                                let is_provided_by_extension = context_server_store
                                    .read(cx)
                                    .configuration_for_server(&context_server_id)
                                    .as_ref()
                                    .map(|config| {
                                        matches!(
                                            config.as_ref(),
                                            ContextServerConfiguration::Extension { .. }
                                        )
                                    })
                                    .unwrap_or(false);

                                let uninstall_extension_task = match (
                                    is_provided_by_extension,
                                    resolve_extension_for_context_server(&context_server_id, cx),
                                ) {
                                    (true, Some((id, manifest))) => {
                                        if extension_only_provides_context_server(manifest.as_ref())
                                        {
                                            ExtensionStore::global(cx).update(cx, |store, cx| {
                                                store.uninstall_extension(id, cx)
                                            })
                                        } else {
                                            workspace.update(cx, |workspace, cx| {
                                                show_unable_to_uninstall_extension_with_context_server(workspace, context_server_id.clone(), cx);
                                            }).log_err();
                                            Task::ready(Ok(()))
                                        }
                                    }
                                    _ => Task::ready(Ok(())),
                                };

                                cx.spawn({
                                    let fs = fs.clone();
                                    let context_server_id = context_server_id.clone();
                                    async move |cx| {
                                        uninstall_extension_task.await?;
                                        cx.update(|cx| {
                                            update_settings_file::<ProjectSettings>(
                                                fs.clone(),
                                                cx,
                                                {
                                                    let context_server_id =
                                                        context_server_id.clone();
                                                    move |settings, _| {
                                                        settings
                                                            .context_servers
                                                            .remove(&context_server_id.0);
                                                    }
                                                },
                                            )
                                        })
                                    }
                                })
                                .detach_and_log_err(cx);
                            }
                        })
                    }))
                }
            });

        v_flex()
            .id(item_id.clone())
            .border_1()
            .rounded_md()
            .border_color(border_color)
            .bg(cx.theme().colors().background.opacity(0.2))
            .overflow_hidden()
            .child(
                h_flex()
                    .p_1()
                    .justify_between()
                    .when(
                        error.is_some() || are_tools_expanded && tool_count >= 1,
                        |element| element.border_b_1().border_color(border_color),
                    )
                    .child(
                        h_flex()
                            .child(
                                Disclosure::new(
                                    "tool-list-disclosure",
                                    are_tools_expanded || error.is_some(),
                                )
                                .disabled(tool_count == 0)
                                .on_click(cx.listener({
                                    let context_server_id = context_server_id.clone();
                                    move |this, _event, _window, _cx| {
                                        let is_open = this
                                            .expanded_context_server_tools
                                            .entry(context_server_id.clone())
                                            .or_insert(false);

                                        *is_open = !*is_open;
                                    }
                                })),
                            )
                            .child(
                                h_flex()
                                    .id(SharedString::from(format!("tooltip-{}", item_id)))
                                    .h_full()
                                    .w_3()
                                    .mx_1()
                                    .justify_center()
                                    .tooltip(Tooltip::text(tooltip_text))
                                    .child(status_indicator),
                            )
                            .child(Label::new(item_id).ml_0p5())
                            .child(
                                div()
                                    .id("extension-source")
                                    .mt_0p5()
                                    .mx_1()
                                    .tooltip(Tooltip::text(source_tooltip))
                                    .child(
                                        Icon::new(source_icon)
                                            .size(IconSize::Small)
                                            .color(Color::Muted),
                                    ),
                            )
                            .when(is_running, |this| {
                                this.child(
                                    Label::new(if tool_count == 1 {
                                        SharedString::from("1 tool")
                                    } else {
                                        SharedString::from(format!("{} tools", tool_count))
                                    })
                                    .color(Color::Muted)
                                    .size(LabelSize::Small),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(context_server_configuration_menu)
                            .child(
                                Switch::new("context-server-switch", is_running.into())
                                    .color(SwitchColor::Accent)
                                    .on_click({
                                        let context_server_manager =
                                            self.context_server_store.clone();
                                        let context_server_id = context_server_id.clone();
                                        let fs = self.fs.clone();

                                        move |state, _window, cx| {
                                            let is_enabled = match state {
                                                ToggleState::Unselected
                                                | ToggleState::Indeterminate => {
                                                    context_server_manager.update(
                                                        cx,
                                                        |this, cx| {
                                                            this.stop_server(
                                                                &context_server_id,
                                                                cx,
                                                            )
                                                            .log_err();
                                                        },
                                                    );
                                                    false
                                                }
                                                ToggleState::Selected => {
                                                    context_server_manager.update(
                                                        cx,
                                                        |this, cx| {
                                                            if let Some(server) =
                                                                this.get_server(&context_server_id)
                                                            {
                                                                this.start_server(server, cx);
                                                            }
                                                        },
                                                    );
                                                    true
                                                }
                                            };
                                            update_settings_file::<ProjectSettings>(
                                                fs.clone(),
                                                cx,
                                                {
                                                    let context_server_id =
                                                        context_server_id.clone();

                                                    move |settings, _| {
                                                        settings
                                                            .context_servers
                                                            .entry(context_server_id.0)
                                                            .or_insert_with(|| {
                                                                ContextServerSettings::Extension {
                                                                    enabled: is_enabled,
                                                                    settings: serde_json::json!({}),
                                                                }
                                                            })
                                                            .set_enabled(is_enabled);
                                                    }
                                                },
                                            );
                                        }
                                    }),
                            ),
                    ),
            )
            .map(|parent| {
                if let Some(error) = error {
                    return parent.child(
                        h_flex()
                            .p_2()
                            .gap_2()
                            .items_start()
                            .child(
                                h_flex()
                                    .flex_none()
                                    .h(window.line_height() / 1.6_f32)
                                    .justify_center()
                                    .child(
                                        Icon::new(IconName::XCircle)
                                            .size(IconSize::XSmall)
                                            .color(Color::Error),
                                    ),
                            )
                            .child(
                                div().w_full().child(
                                    Label::new(error)
                                        .buffer_font(cx)
                                        .color(Color::Muted)
                                        .size(LabelSize::Small),
                                ),
                            ),
                    );
                }

                if !are_tools_expanded || tools.is_empty() {
                    return parent;
                }

                parent.child(v_flex().py_1p5().px_1().gap_1().children(
                    tools.into_iter().enumerate().map(|(ix, tool)| {
                        h_flex()
                            .id(("tool-item", ix))
                            .px_1()
                            .gap_2()
                            .justify_between()
                            .hover(|style| style.bg(cx.theme().colors().element_hover))
                            .rounded_sm()
                            .child(
                                Label::new(tool.name())
                                    .buffer_font(cx)
                                    .size(LabelSize::Small),
                            )
                            .child(
                                Icon::new(IconName::Info)
                                    .size(IconSize::Small)
                                    .color(Color::Ignored),
                            )
                            .tooltip(Tooltip::text(tool.description()))
                    }),
                ))
            })
    }
}

impl Render for AgentConfiguration {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("assistant-configuration")
            .key_context("AgentConfiguration")
            .track_focus(&self.focus_handle(cx))
            .relative()
            .size_full()
            .pb_8()
            .bg(cx.theme().colors().panel_background)
            .child(
                v_flex()
                    .id("assistant-configuration-content")
                    .track_scroll(&self.scroll_handle)
                    .size_full()
                    .overflow_y_scroll()
                    .child(self.render_general_settings_section(cx))
                    .child(self.render_task_sync_section(window, cx))
                    .child(self.render_context_servers_section(window, cx))
                    .child(self.render_provider_configuration_section(cx)),
            )
            .child(
                div()
                    .id("assistant-configuration-scrollbar")
                    .occlude()
                    .absolute()
                    .right(px(3.))
                    .top_0()
                    .bottom_0()
                    .pb_6()
                    .w(px(12.))
                    .cursor_default()
                    .on_mouse_move(cx.listener(|_, _, _window, cx| {
                        cx.notify();
                        cx.stop_propagation()
                    }))
                    .on_hover(|_, _window, cx| {
                        cx.stop_propagation();
                    })
                    .on_any_mouse_down(|_, _window, cx| {
                        cx.stop_propagation();
                    })
                    .on_scroll_wheel(cx.listener(|_, _, _window, cx| {
                        cx.notify();
                    }))
                    .children(Scrollbar::vertical(self.scrollbar_state.clone())),
            )
    }
}

fn extension_only_provides_context_server(manifest: &ExtensionManifest) -> bool {
    manifest.context_servers.len() == 1
        && manifest.themes.is_empty()
        && manifest.icon_themes.is_empty()
        && manifest.languages.is_empty()
        && manifest.grammars.is_empty()
        && manifest.language_servers.is_empty()
        && manifest.slash_commands.is_empty()
        && manifest.indexed_docs_providers.is_empty()
        && manifest.snippets.is_none()
        && manifest.debug_locators.is_empty()
}

pub(crate) fn resolve_extension_for_context_server(
    id: &ContextServerId,
    cx: &App,
) -> Option<(Arc<str>, Arc<ExtensionManifest>)> {
    ExtensionStore::global(cx)
        .read(cx)
        .installed_extensions()
        .iter()
        .find(|(_, entry)| entry.manifest.context_servers.contains_key(&id.0))
        .map(|(id, entry)| (id.clone(), entry.manifest.clone()))
}

// This notification appears when trying to delete
// an MCP server extension that not only provides
// the server, but other things, too, like language servers and more.
fn show_unable_to_uninstall_extension_with_context_server(
    workspace: &mut Workspace,
    id: ContextServerId,
    cx: &mut App,
) {
    let workspace_handle = workspace.weak_handle();
    let context_server_id = id.clone();

    let status_toast = StatusToast::new(
        format!(
            "The {} extension provides more than just the MCP server. Proceed to uninstall anyway?",
            id.0
        ),
        cx,
        move |this, _cx| {
            let workspace_handle = workspace_handle.clone();
            let context_server_id = context_server_id.clone();

            this.icon(ToastIcon::new(IconName::Warning).color(Color::Warning))
                .dismiss_button(true)
                .action("Uninstall", move |_, _cx| {
                    if let Some((extension_id, _)) =
                        resolve_extension_for_context_server(&context_server_id, _cx)
                    {
                        ExtensionStore::global(_cx).update(_cx, |store, cx| {
                            store
                                .uninstall_extension(extension_id, cx)
                                .detach_and_log_err(cx);
                        });

                        workspace_handle
                            .update(_cx, |workspace, cx| {
                                let fs = workspace.app_state().fs.clone();
                                cx.spawn({
                                    let context_server_id = context_server_id.clone();
                                    async move |_workspace_handle, cx| {
                                        cx.update(|cx| {
                                            update_settings_file::<ProjectSettings>(
                                                fs,
                                                cx,
                                                move |settings, _| {
                                                    settings
                                                        .context_servers
                                                        .remove(&context_server_id.0);
                                                },
                                            );
                                        })?;
                                        anyhow::Ok(())
                                    }
                                })
                                .detach_and_log_err(cx);
                            })
                            .log_err();
                    }
                })
        },
    );

    workspace.toggle_status_toast(status_toast, cx);
}
