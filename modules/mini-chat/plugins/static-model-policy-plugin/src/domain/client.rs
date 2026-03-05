use async_trait::async_trait;
use mini_chat_sdk::{
    MiniChatModelPolicyPluginClientV1, MiniChatModelPolicyPluginError, PolicySnapshot,
    PolicyVersionInfo, UserLimits,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::service::Service;

#[async_trait]
impl MiniChatModelPolicyPluginClientV1 for Service {
    async fn get_current_policy_version(
        &self,
        tenant_id: Uuid,
    ) -> Result<PolicyVersionInfo, MiniChatModelPolicyPluginError> {
        Ok(PolicyVersionInfo {
            tenant_id,
            policy_version: 1,
            generated_at: OffsetDateTime::now_utc(),
        })
    }

    async fn get_policy_snapshot(
        &self,
        tenant_id: Uuid,
        policy_version: u64,
    ) -> Result<PolicySnapshot, MiniChatModelPolicyPluginError> {
        if policy_version != 1 {
            return Err(MiniChatModelPolicyPluginError::NotFound);
        }
        Ok(PolicySnapshot {
            tenant_id,
            policy_version,
            model_catalog: self.catalog.clone(),
            kill_switches: self.kill_switches.clone(),
        })
    }

    async fn get_user_limits(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        policy_version: u64,
    ) -> Result<UserLimits, MiniChatModelPolicyPluginError> {
        if policy_version != 1 {
            return Err(MiniChatModelPolicyPluginError::NotFound);
        }
        Ok(UserLimits {
            tenant_id,
            user_id,
            policy_version,
            standard: self.default_standard_limits.clone(),
            premium: self.default_premium_limits.clone(),
        })
    }
}
