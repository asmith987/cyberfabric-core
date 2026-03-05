use mini_chat_sdk::{KillSwitches, ModelCatalogEntry, ModelTier, TierLimits};
use serde::Deserialize;

/// Plugin configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StaticMiniChatPolicyPluginConfig {
    /// Vendor name for GTS instance registration.
    pub vendor: String,

    /// Plugin priority (lower = higher priority).
    pub priority: i16,

    /// Static model catalog entries.
    pub model_catalog: Vec<ModelCatalogEntry>,

    /// Static kill switches (all disabled by default).
    pub kill_switches: KillSwitches,

    /// Static per-user tier limits (used for all users).
    pub default_standard_limits: TierLimits,
    pub default_premium_limits: TierLimits,
}

impl Default for StaticMiniChatPolicyPluginConfig {
    fn default() -> Self {
        Self {
            vendor: "hyperspot".to_owned(),
            priority: 100,
            model_catalog: vec![
                ModelCatalogEntry {
                    model_id: "gpt-5.2".to_owned(),
                    display_name: "GPT-5.2".to_owned(),
                    tier: ModelTier::Premium,
                    global_enabled: true,
                    is_default: true,
                    input_tokens_credit_multiplier_micro: 3_000_000,
                    output_tokens_credit_multiplier_micro: 15_000_000,
                    multimodal_capabilities: vec!["VISION_INPUT".to_owned()],
                    context_window: 128_000,
                    max_output_tokens: 8_192,
                    description: "Most capable model".to_owned(),
                    provider_display_name: "OpenAI".to_owned(),
                    multiplier_display: "3x".to_owned(),
                },
                ModelCatalogEntry {
                    model_id: "gpt-5-mini".to_owned(),
                    display_name: "GPT-5 Mini".to_owned(),
                    tier: ModelTier::Standard,
                    global_enabled: true,
                    is_default: true,
                    input_tokens_credit_multiplier_micro: 1_000_000,
                    output_tokens_credit_multiplier_micro: 3_000_000,
                    multimodal_capabilities: vec![],
                    context_window: 128_000,
                    max_output_tokens: 4_096,
                    description: "Fast and efficient model".to_owned(),
                    provider_display_name: "OpenAI".to_owned(),
                    multiplier_display: "1x".to_owned(),
                },
            ],
            kill_switches: KillSwitches::default(),
            default_standard_limits: TierLimits {
                limit_daily_credits_micro: 100_000_000,
                limit_monthly_credits_micro: 1_000_000_000,
            },
            default_premium_limits: TierLimits {
                limit_daily_credits_micro: 50_000_000,
                limit_monthly_credits_micro: 500_000_000,
            },
        }
    }
}
