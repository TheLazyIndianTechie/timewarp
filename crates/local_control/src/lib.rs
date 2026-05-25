//! Shared protocol, discovery, authentication, and client types for local Warp control.
//!
//! The `local_control` crate is intentionally UI-agnostic so the Warp app and
//! `warpctrl` CLI can share the same wire envelopes, action catalog, discovery
//! records, selectors, and credential validation rules.
pub mod auth;
pub mod catalog;
pub mod client;
pub mod discovery;
pub mod protocol;
pub mod selection;
pub mod selectors;

pub use auth::{
    AuthToken, AuthenticatedUserGrant, CredentialGrant, CredentialRequest, ScopedCredential,
};
pub use catalog::{
    ActionImplementationStatus, ActionKind, ActionMetadata, ActionParameterSpec, ActionResultSpec,
    AuthenticatedUserRequirement, EXCLUDED_FILE_CONTENT_ACTION_NAMES, InvocationContext,
    PermissionCategory, RiskTier, StateDataCategory, TargetScope,
};
pub use discovery::{
    ControlEndpoint, CredentialBrokerReference, InstanceId, InstanceRecord, RegisteredInstance,
    discovery_dir,
};
pub use protocol::{
    Action, ActionParams, ApiKeySource, BlockOutputFormat, ControlError, ControlResponse,
    ControlResult, Direction, ErrorCode, ErrorResponseEnvelope, ExecutionContextProof,
    FileOpenParams, InputMode, PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope,
    TabActivationMode, TabCloseMode, TabCreateParams, TabType,
};
pub use selectors::{
    BlockSelector, BlockTarget, DriveObjectId, DriveObjectTarget, DriveObjectType, FileTarget,
    InstanceTarget, PaneSelector, PaneTarget, ProjectTarget, SessionSelector, SessionTarget,
    TabSelector, TabTarget, TargetSelector, WindowSelector, WindowTarget,
};
