//! Auth subcommands for `warpctrl`: status, login, and API-key management.
//!
//! These commands expose the authenticated scripting status and allow users to
//! configure external API-key identity without handling raw key material in
//! command output, logs, shell completions, or discovery records.
use clap::{Args, Subcommand};

use crate::local_control::TargetArgs;

/// Authentication and scripting identity commands.
#[derive(Debug, Clone, Subcommand)]
pub enum AuthCommand {
    /// Report authenticated scripting status for the selected Warp app.
    ///
    /// Prints whether the app user is logged in, whether authenticated scripting
    /// grants are enabled, and the configured API-key subject metadata without
    /// exposing raw key material.
    Status(TargetArgs),

    /// Focus the selected Warp app's sign-in UI for interactive login.
    ///
    /// Opens the normal Warp sign-in flow. Use this to log in interactively
    /// before issuing authenticated-user control actions.
    Login(TargetArgs),

    /// Manage external scripting API keys.
    #[command(subcommand)]
    ApiKey(ApiKeySubcommand),
}

/// API-key management subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum ApiKeySubcommand {
    /// Store or reference an external Warp scripting API key.
    ///
    /// The raw key is read from the environment variable or stdin and stored in
    /// platform secure storage where available. It is never written to logs,
    /// discovery records, shell completions, or command output.
    Set(ApiKeySetArgs),

    /// Show the subject and scope metadata for the stored scripting API key.
    ///
    /// Does not print the raw key. Prints the opaque key ID, bound Warp user
    /// subject, and configured permission scopes.
    Status(TargetArgs),

    /// Delete the locally stored API key reference and revoke it server-side where supported.
    Revoke(TargetArgs),
}

/// Arguments for `warpctrl auth api-key set`.
#[derive(Debug, Clone, Args)]
#[group(required = true, multiple = false)]
pub struct ApiKeySourceArgs {
    /// Read the API key from this environment variable.
    ///
    /// The variable must hold the raw key value. Use a secret manager to
    /// inject the variable at runtime rather than setting it in a shell profile.
    #[arg(long = "key-env", value_name = "ENV_VAR")]
    pub key_env: Option<String>,

    /// Read the API key from stdin.
    ///
    /// Pipe the key from a secret manager or password manager. The key is read
    /// once and stored; it is not echoed or logged.
    #[arg(long = "key-stdin")]
    pub key_stdin: bool,
}

/// Arguments for `warpctrl auth api-key set`.
#[derive(Debug, Clone, Args)]
pub struct ApiKeySetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[command(flatten)]
    pub source: ApiKeySourceArgs,
}
