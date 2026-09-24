//! # akumo-provider-aws
//!
//! The first real Provider Port adapter (ADR-0002): AWS. **The AWS SDK is confined to this crate**
//! — the core stays provider-blind (NFR-EXT1, enforced by `xtask dep-lint`), so adding AWS changes
//! zero core lines (EXR-3).
//!
//! v1 is a minimal, honest slice: STS identity, IAM user/role enumeration, and the
//! create/delete-access-key action pair (matching the example technique). Credentials come from the
//! standard AWS provider chain (env vars, shared config/profile, assumed roles), which covers the
//! multiple credential input types of FR-B3/B4. Pagination, more services, and provider-assisted
//! validation (AWS `DryRun` / IAM policy simulator) are additive follow-ups.

#![forbid(unsafe_code)]

mod mapper;

use async_trait::async_trait;

use akumo_domain::error::{AkumoError, Result};
use akumo_domain::graph::Assertion;
use akumo_domain::ids::{ProviderId, Region};
use akumo_domain::ports::provider::{
    ActionExecutor, GraphMapper, IdentityResolver, MetadataProvider, Provider, ResourceEnumerator,
    TelemetryCollector,
};
use akumo_domain::principal::{Principal, PrincipalKind};
use akumo_domain::seam::{
    ActionDescriptor, ActionResult, CapabilityGrant, EnumerationDescriptor, RawResponse,
};

pub use akumo_domain as domain;

/// The AWS Provider Port adapter.
pub struct AwsProvider {
    provider_id: ProviderId,
    regions: Vec<Region>,
    sts: aws_sdk_sts::Client,
    iam: aws_sdk_iam::Client,
}

impl AwsProvider {
    /// Connect using the standard AWS provider chain (env / shared config / assumed role). The
    /// active region (if any) is exposed via [`MetadataProvider`].
    pub async fn connect() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let regions = config
            .region()
            .map(|r| Region::new(r.to_string()))
            .into_iter()
            .collect();
        Ok(Self {
            provider_id: ProviderId::new("aws"),
            regions,
            sts: aws_sdk_sts::Client::new(&config),
            iam: aws_sdk_iam::Client::new(&config),
        })
    }
}

/// Classify an AWS SDK error: access/authorization failures become the first-class `AccessDenied`
/// gap (FR-C6); everything else is a provider error.
fn sdk_error(context: &str, error: impl std::error::Error) -> AkumoError {
    let message = error.to_string();
    if message.contains("AccessDenied")
        || message.contains("not authorized")
        || message.contains("UnauthorizedOperation")
        || message.contains("Forbidden")
    {
        AkumoError::AccessDenied(format!("{context}: {message}"))
    } else {
        AkumoError::Provider(format!("{context}: {message}"))
    }
}

fn principal_kind_from_arn(arn: &str) -> PrincipalKind {
    if arn.contains(":role/") || arn.contains(":assumed-role/") {
        PrincipalKind::Role
    } else if arn.contains(":user/") {
        PrincipalKind::User
    } else {
        PrincipalKind::ServiceIdentity
    }
}

fn required_str<'a>(
    params: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
    op: &str,
) -> Result<&'a str> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| AkumoError::Validation(format!("{op} requires a string '{key}' parameter")))
}

#[async_trait]
impl IdentityResolver for AwsProvider {
    async fn resolve_current_principal(&self) -> Result<Principal> {
        let out = self
            .sts
            .get_caller_identity()
            .send()
            .await
            .map_err(|e| sdk_error("sts:GetCallerIdentity", e))?;
        let arn = out.arn().unwrap_or_default().to_string();
        let kind = principal_kind_from_arn(&arn);
        Ok(Principal::new(arn, kind, self.provider_id.clone()))
    }
}

#[async_trait]
impl ResourceEnumerator for AwsProvider {
    async fn execute(&self, descriptor: &EnumerationDescriptor) -> Result<RawResponse> {
        match descriptor.key().as_str() {
            "iam.ListUsers" => {
                let out = self
                    .iam
                    .list_users()
                    .send()
                    .await
                    .map_err(|e| sdk_error("iam:ListUsers", e))?;
                let items: Vec<serde_json::Value> = out
                    .users()
                    .iter()
                    .map(|u| {
                        serde_json::json!({
                            "arn": u.arn(),
                            "name": u.user_name(),
                            "type": "user",
                        })
                    })
                    .collect();
                Ok(RawResponse {
                    raw: serde_json::json!({ "kind": "principals", "items": items }),
                })
            }
            "iam.ListRoles" => {
                let out = self
                    .iam
                    .list_roles()
                    .send()
                    .await
                    .map_err(|e| sdk_error("iam:ListRoles", e))?;
                let items: Vec<serde_json::Value> = out
                    .roles()
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "arn": r.arn(),
                            "name": r.role_name(),
                            "type": "role",
                        })
                    })
                    .collect();
                Ok(RawResponse {
                    raw: serde_json::json!({ "kind": "principals", "items": items }),
                })
            }
            other => Err(AkumoError::Unsupported(format!(
                "AWS enumeration '{other}' is not implemented in v1"
            ))),
        }
    }
}

impl GraphMapper for AwsProvider {
    fn map(&self, response: &RawResponse) -> Result<Vec<Assertion>> {
        Ok(mapper::map_response(response))
    }
}

#[async_trait]
impl ActionExecutor for AwsProvider {
    async fn execute(
        &self,
        descriptor: &ActionDescriptor,
        grant: &CapabilityGrant,
    ) -> Result<ActionResult> {
        let key = descriptor.key();
        if !grant.permits(&key) {
            return Err(AkumoError::AccessDenied(format!(
                "capability not granted: {key}"
            )));
        }
        match key.as_str() {
            "iam.CreateAccessKey" => {
                let user = required_str(&descriptor.params, "UserName", "iam:CreateAccessKey")?;
                let out = self
                    .iam
                    .create_access_key()
                    .user_name(user)
                    .send()
                    .await
                    .map_err(|e| sdk_error("iam:CreateAccessKey", e))?;
                // Only the key id is returned — the secret is loot and is never surfaced (NFR-SEC2).
                let key_id = out
                    .access_key()
                    .map(|k| k.access_key_id().to_string())
                    .unwrap_or_default();
                Ok(ActionResult {
                    raw: serde_json::json!({ "AccessKeyId": key_id }),
                })
            }
            "iam.DeleteAccessKey" => {
                let user = required_str(&descriptor.params, "UserName", "iam:DeleteAccessKey")?;
                let key_id =
                    required_str(&descriptor.params, "AccessKeyId", "iam:DeleteAccessKey")?;
                self.iam
                    .delete_access_key()
                    .user_name(user)
                    .access_key_id(key_id)
                    .send()
                    .await
                    .map_err(|e| sdk_error("iam:DeleteAccessKey", e))?;
                Ok(ActionResult {
                    raw: serde_json::json!({}),
                })
            }
            other => Err(AkumoError::Unsupported(format!(
                "AWS action '{other}' is not implemented in v1"
            ))),
        }
    }
}

impl MetadataProvider for AwsProvider {
    fn provider_id(&self) -> ProviderId {
        self.provider_id.clone()
    }

    fn regions(&self) -> Vec<Region> {
        self.regions.clone()
    }
}

#[async_trait]
impl TelemetryCollector for AwsProvider {}

impl Provider for AwsProvider {
    fn id(&self) -> ProviderId {
        self.provider_id.clone()
    }
    fn identity(&self) -> &dyn IdentityResolver {
        self
    }
    fn enumerator(&self) -> &dyn ResourceEnumerator {
        self
    }
    fn actions(&self) -> &dyn ActionExecutor {
        self
    }
    fn mapper(&self) -> &dyn GraphMapper {
        self
    }
    fn metadata(&self) -> &dyn MetadataProvider {
        self
    }
    fn telemetry(&self) -> &dyn TelemetryCollector {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arn_kind_classification() {
        assert_eq!(
            principal_kind_from_arn("arn:aws:iam::1:role/admin"),
            PrincipalKind::Role
        );
        assert_eq!(
            principal_kind_from_arn("arn:aws:iam::1:user/alice"),
            PrincipalKind::User
        );
        assert_eq!(
            principal_kind_from_arn("arn:aws:sts::1:assumed-role/x/y"),
            PrincipalKind::Role
        );
    }
}
