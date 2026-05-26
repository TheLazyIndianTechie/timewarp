//! Implementations for user-facing `warpctrl` command groups.
use local_control::protocol::{
    Action, ActionImplementationStatus, ActionKind, ActionMetadata, ControlError, ErrorCode,
    RequestEnvelope,
};
use local_control::selection::select_instance;
use serde::Serialize;
use serde_json::json;

use crate::agent::OutputFormat;
use crate::local_control::output::{write_json, write_json_line};
use crate::local_control::selectors::instance_selector;
use crate::local_control::{
    ActionCatalogCommand, AppCommand, CapabilityCommand, CatalogFilterArgs, InstanceCommand,
    TabCommand, TargetArgs,
};

/// Display-oriented projection of a discoverable Warp instance.
#[derive(Serialize)]
struct InstanceSummary {
    instance_id: String,
    pid: u32,
    channel: String,
    app_id: String,
    app_version: Option<String>,
    started_at: String,
    endpoint: Option<local_control::discovery::ControlEndpoint>,
    outside_warp_control_enabled: bool,
    actions: Vec<ActionMetadata>,
}

#[derive(Serialize)]
struct CatalogActionSummary {
    name: String,
    implementation_status: ActionImplementationStatus,
    requires_authenticated_user: bool,
    target_scope: local_control::protocol::TargetScope,
    permission_category: local_control::protocol::PermissionCategory,
}

impl From<local_control::discovery::InstanceRecord> for InstanceSummary {
    fn from(record: local_control::discovery::InstanceRecord) -> Self {
        Self {
            instance_id: record.instance_id.0,
            pid: record.pid,
            channel: record.channel,
            app_id: record.app_id,
            app_version: record.app_version,
            started_at: record.started_at.to_rfc3339(),
            endpoint: record.endpoint,
            outside_warp_control_enabled: record.outside_warp_control_enabled,
            actions: record.actions,
        }
    }
}

impl From<ActionMetadata> for CatalogActionSummary {
    fn from(metadata: ActionMetadata) -> Self {
        Self {
            name: metadata.name,
            implementation_status: metadata.implementation_status,
            requires_authenticated_user: metadata.requires_authenticated_user,
            target_scope: metadata.target_scope,
            permission_category: metadata.permission_category,
        }
    }
}

pub(super) fn run_instance_command(
    command: InstanceCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        InstanceCommand::List => {
            let summaries = local_control::discovery::list_instances()
                .into_iter()
                .map(InstanceSummary::from)
                .collect::<Vec<_>>();
            match output_format {
                OutputFormat::Json => write_json(&summaries),
                OutputFormat::Ndjson => {
                    for summary in summaries {
                        write_json_line(&summary)?;
                    }
                    Ok(())
                }
                OutputFormat::Pretty | OutputFormat::Text => {
                    for summary in summaries {
                        let endpoint = summary
                            .endpoint
                            .as_ref()
                            .map(|endpoint| format!("{}:{}", endpoint.host, endpoint.port))
                            .unwrap_or_else(|| "outside_warp_disabled".to_owned());
                        println!(
                            "{}\tpid={}\t{}\t{}",
                            summary.instance_id, summary.pid, summary.channel, endpoint
                        );
                    }
                    Ok(())
                }
            }
        }
        InstanceCommand::Inspect(args) => {
            run_action(args, ActionKind::InstanceInspect, json!({}), output_format)
        }
    }
}

pub(super) fn run_app_command(
    command: AppCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        AppCommand::Ping(args) => run_action(args, ActionKind::AppPing, json!({}), output_format),
        AppCommand::Version(args) => {
            run_action(args, ActionKind::AppVersion, json!({}), output_format)
        }
        AppCommand::Active(args) => run_action(args, ActionKind::AppActive, json!({}), output_format),
        AppCommand::Focus(args) => run_action(args, ActionKind::AppFocus, json!({}), output_format),
    }
}
pub(super) fn run_tab_command(
    command: TabCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        TabCommand::Create(args) => {
            run_action(args, ActionKind::TabCreate, json!({}), output_format)
        }
    }
}

pub(super) fn run_action_catalog_command(
    command: ActionCatalogCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        ActionCatalogCommand::List(args) => render_catalog_list(args, output_format),
        ActionCatalogCommand::Inspect { action } => {
            render_catalog_metadata(metadata_for_action_name(&action)?, output_format)
        }
    }
}

pub(super) fn run_capability_command(
    command: CapabilityCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        CapabilityCommand::List(args) => render_catalog_list(args, output_format),
        CapabilityCommand::Inspect { action } => {
            render_catalog_metadata(metadata_for_action_name(&action)?, output_format)
        }
    }
}

fn render_catalog_list(
    args: CatalogFilterArgs,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    let metadata = ActionKind::ALL
        .iter()
        .copied()
        .map(ActionKind::metadata)
        .filter(|metadata| {
            if args.implemented_only {
                metadata.implementation_status == ActionImplementationStatus::Implemented
            } else if args.stubs_only {
                metadata.implementation_status == ActionImplementationStatus::Stub
            } else {
                true
            }
        })
        .collect::<Vec<_>>();
    match output_format {
        OutputFormat::Json => write_json(&metadata),
        OutputFormat::Ndjson => {
            for metadata in metadata {
                write_json_line(&metadata)?;
            }
            Ok(())
        }
        OutputFormat::Pretty | OutputFormat::Text => {
            for summary in metadata.into_iter().map(CatalogActionSummary::from) {
                println!(
                    "{}\tstatus={:?}\tscope={:?}\tpermission={:?}\tauthenticated_user={}",
                    summary.name,
                    summary.implementation_status,
                    summary.target_scope,
                    summary.permission_category,
                    summary.requires_authenticated_user
                );
            }
            Ok(())
        }
    }
}

fn render_catalog_metadata(
    metadata: ActionMetadata,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match output_format {
        OutputFormat::Json => write_json(&metadata),
        OutputFormat::Ndjson => write_json_line(&metadata),
        OutputFormat::Pretty | OutputFormat::Text => write_json(&metadata),
    }
}

fn metadata_for_action_name(action: &str) -> Result<ActionMetadata, ControlError> {
    ActionKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.as_str() == action)
        .map(ActionKind::metadata)
        .ok_or_else(|| {
            ControlError::with_details(
                ErrorCode::NotAllowlisted,
                format!("{action} is not in the public warpctrl action catalog"),
                "Use `warpctrl action list` to inspect allowlisted actions.",
            )
        })
}

fn run_action(
    args: TargetArgs,
    action: ActionKind,
    params: serde_json::Value,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    let records = local_control::discovery::list_instances();
    let selector = instance_selector(args);
    let instance = select_instance(&records, &selector)?;
    let request = RequestEnvelope::new(Action {
        kind: action,
        params,
    });
    let response = local_control::client::send_request(&instance, &request)?;
    let local_control::protocol::ControlResponse::Ok { data } = response.response else {
        return Err(ControlError::new(
            ErrorCode::Internal,
            "local-control request failed without an error payload",
        ));
    };
    match output_format {
        OutputFormat::Json => write_json(&data),
        OutputFormat::Ndjson => write_json_line(&data),
        OutputFormat::Pretty | OutputFormat::Text => write_json(&data),
    }
}
