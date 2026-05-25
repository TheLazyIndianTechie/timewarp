//! Authenticated scripting identity types for local Warp control.
//!
//! This module defines the structures used for the two authenticated scripting
//! paths described in the security architecture:
//!
//! 1. **Verified Warp-terminal path**: when `warpctrl` is launched inside a
//!    Warp-managed terminal session the app-issued proof broker can mint a
//!    terminal-session-bound scripting grant. The broker infrastructure is
//!    scaffolded here but not yet operational; the credential endpoint rejects
//!    `InsideWarp` requests until the proof verification path is complete.
//!
//! 2. **External API-key path**: when `warpctrl` is launched outside Warp a
//!    Warp-issued scripting API key can be exchanged for a short-lived signed
//!    identity assertion. The exchange and storage stubs are here; the actual
//!    server-side exchange is wired up when external authenticated grants are
//!    enabled in Settings > Scripting.
//!
//! Raw API keys are never stored in this module, logged, or written to
//! discovery records. Only opaque key identifiers and subject metadata are
//! retained after an exchange or validation step.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Permission scope carried by a scripting grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptingScope {
    LocalControlRead,
    LocalControlMutateAppState,
    LocalControlMutateMetadataConfiguration,
    LocalControlMutateUnderlyingData,
}

/// How a scripting grant was obtained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ScriptingIdentitySource {
    VerifiedWarpTerminal { session_id: String },
    ExternalApiKey { key_id: String },
}

/// Authenticated scripting grant attached to a local-control credential.
///
/// A scripting grant proves that the caller holds either a verified
/// Warp-terminal session context or a valid external API key with appropriate
/// scopes. Actions that require authenticated scripting authority check this
/// grant before selector resolution or dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptingGrant {
    pub source: ScriptingIdentitySource,
    pub subject: String,
    pub scopes: Vec<ScriptingScope>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ScriptingGrant {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    pub fn has_scope(&self, scope: &ScriptingScope) -> bool {
        self.scopes.contains(scope)
    }
}

/// Placeholder registry entry for a verified Warp-terminal session.
///
/// When the app creates or Warpifies a terminal session it will record a
/// `TerminalProofSession` that binds the session to an app instance, expiry,
/// and revocation state. The CLI sends `ExecutionContextProof::VerifiedWarpTerminal`
/// and the broker verifies the proof against this registry before minting a
/// scripting grant. The registry and verification logic are not yet operational;
/// this struct documents the intended shape.
#[derive(Debug, Clone)]
pub struct TerminalProofSession {
    pub session_id: String,
    pub instance_id: String,
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
}

/// Reference to a stored API key in platform secure storage.
///
/// Raw key material is never held in memory beyond the exchange call. Only
/// the opaque `key_id` returned by the exchange and the Warp user subject
/// bound to the key are persisted for grant validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyStorageRef {
    pub key_id: String,
    pub subject: String,
    pub scopes: Vec<ScriptingScope>,
}

/// Status of a stored or configured external scripting API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyStatus {
    NotConfigured,
    Configured {
        key_id: String,
        subject: String,
        scopes: Vec<ScriptingScope>,
    },
}

/// Summary of the authenticated scripting status for `warpctrl auth status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthStatusSummary {
    pub app_user_logged_in: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_user_subject: Option<String>,
    pub outside_warp_authenticated_grants_enabled: bool,
    pub api_key_status: ApiKeyStatus,
}

#[cfg(test)]
#[path = "scripting_tests.rs"]
mod tests;
