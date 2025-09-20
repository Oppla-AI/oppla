use std::sync::Arc;

use collections::IndexMap;
use gpui::SharedString;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod builtin_profiles {
    use super::AgentProfileId;

    pub const WRITE: &str = "write";
    pub const ASK: &str = "ask";
    pub const MINIMAL: &str = "minimal";
    pub const ARCHITECT: &str = "architect";
    pub const DEVOPS: &str = "devops";
    pub const MARKETING_SPECIALIST: &str = "marketing-specialist";
    pub const PRODUCT_ENGINEER: &str = "product-engineer";

    pub fn is_builtin(profile_id: &AgentProfileId) -> bool {
        matches!(
            profile_id.as_str(),
            WRITE | ASK | MINIMAL | ARCHITECT | DEVOPS | MARKETING_SPECIALIST | PRODUCT_ENGINEER
        )
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentProfileId(pub Arc<str>);

impl AgentProfileId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AgentProfileId {
    fn default() -> Self {
        Self("write".into())
    }
}

/// A profile for the Zed Agent that controls its behavior.
#[derive(Debug, Clone)]
pub struct AgentProfileSettings {
    /// The name of the profile.
    pub name: SharedString,
    /// Optional custom prompt template for this profile.
    pub prompt_template: Option<String>,
    /// Optional role description for additional context.
    pub role_description: Option<String>,
    pub tools: IndexMap<Arc<str>, bool>,
    pub enable_all_context_servers: bool,
    pub context_servers: IndexMap<Arc<str>, ContextServerPreset>,
}

#[derive(Debug, Clone, Default)]
pub struct ContextServerPreset {
    pub tools: IndexMap<Arc<str>, bool>,
}
