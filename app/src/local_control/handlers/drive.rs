use ::local_control::protocol::{
    DriveInspectParams, DriveInspectResult, DriveListParams, DriveListResult, DriveObjectSummary,
    DriveObjectType, TargetSelector, WindowTarget,
};
use ::local_control::{ActionKind, ControlError, ErrorCode};
use serde_json::json;
use warpui::{ModelContext, SingletonEntity, TypedActionView, ViewHandle, WindowId};

use crate::cloud_object::{
    model::persistence::CloudModel, CloudObject, GenericStringObjectFormat, JsonObjectType,
    ObjectType,
};
use crate::drive::folders::CloudFolder;
use crate::drive::items::WarpDriveItemId;
use crate::drive::CloudObjectTypeAndId;
use crate::env_vars::manager::EnvVarCollectionSource;
use crate::env_vars::CloudEnvVarCollection;
use crate::local_control::resolver::require_active_window_id_for_action;
use crate::local_control::LocalControlBridge;
use crate::notebooks::CloudNotebook;
use crate::server::ids::SyncId;
use crate::server::telemetry::SharingDialogSource;
use crate::workflows::CloudWorkflow;
use crate::workspace::{Workspace, WorkspaceAction};

pub(crate) fn drive_list(
    target: &TargetSelector,
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_drive_target(target, ActionKind::DriveList)?;
    let params = action.params_as::<DriveListParams>()?;
    let mut objects = CloudModel::as_ref(ctx)
        .cloud_objects()
        .filter_map(|object| drive_object_summary(object.as_ref()))
        .filter(|summary| {
            params
                .object_type
                .is_none_or(|object_type| summary.object_type == object_type)
        })
        .collect::<Vec<_>>();
    objects.sort_by(|left, right| {
        drive_object_type_rank(left.object_type)
            .cmp(&drive_object_type_rank(right.object_type))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    serde_json::to_value(DriveListResult { objects }).map_err(json_response_error)
}

pub(crate) fn drive_inspect(
    target: &TargetSelector,
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_drive_target(target, ActionKind::DriveInspect)?;
    let params = action.params_as::<DriveInspectParams>()?;
    let object = CloudModel::as_ref(ctx)
        .get_by_uid(&params.id)
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::StaleTarget,
                "drive.inspect could not resolve the requested Drive object id",
            )
        })?;
    drive_object_get_result(object)
        .and_then(|result| serde_json::to_value(result).map_err(json_response_error))
}

pub(crate) fn validate_drive_target(
    target: &TargetSelector,
    action: ActionKind,
) -> Result<(), ControlError> {
    if target.window.is_some()
        || target.tab.is_some()
        || target.pane.is_some()
        || target.session.is_some()
    {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} does not accept window, tab, pane, or session selectors",
                action.as_str()
            ),
        ));
    }
    Ok(())
}

/// Validates that an open-style Drive action only targets the active window or
/// an opaque window id. Tab/pane/session selectors are rejected because Drive
/// open actions operate on app-wide Warp Drive state, not pane state.
fn validate_drive_open_target(
    target: &TargetSelector,
    action: ActionKind,
) -> Result<(), ControlError> {
    if target.tab.is_some() || target.pane.is_some() || target.session.is_some() {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} does not accept tab, pane, or session selectors",
                action.as_str()
            ),
        ));
    }
    if matches!(
        target.window.as_ref(),
        Some(WindowTarget::Index { .. } | WindowTarget::Title { .. })
    ) {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} only supports active and opaque window id selectors",
                action.as_str()
            ),
        ));
    }
    Ok(())
}

pub(crate) fn drive_open(
    target: &TargetSelector,
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_drive_open_target(target, ActionKind::DriveOpen)?;
    let params = action.params_as::<DriveInspectParams>()?;
    let object_type_and_id =
        resolve_cloud_object_type_and_id(&params.id, ActionKind::DriveOpen, ctx)?;
    let window_id = select_window_for_drive_open(ActionKind::DriveOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::DriveOpen, window_id, ctx)?;
    workspace.update(ctx, |workspace, view_ctx| {
        workspace.handle_action(
            &WorkspaceAction::ViewObjectInWarpDrive(WarpDriveItemId::Object(object_type_and_id)),
            view_ctx,
        );
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "action": ActionKind::DriveOpen.as_str(),
        "opened": true,
        "id": params.id,
        "object_type": cloud_object_type_label(object_type_and_id),
        "window_id": window_id.to_string(),
    }))
}

pub(crate) fn drive_notebook_open(
    target: &TargetSelector,
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_drive_open_target(target, ActionKind::DriveNotebookOpen)?;
    let params = action.params_as::<DriveInspectParams>()?;
    let sync_id = resolve_typed_drive_sync_id(
        &params.id,
        ObjectType::Notebook,
        ActionKind::DriveNotebookOpen,
        ctx,
    )?;
    let window_id = select_window_for_drive_open(ActionKind::DriveNotebookOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::DriveNotebookOpen, window_id, ctx)?;
    workspace.update(ctx, |workspace, view_ctx| {
        workspace.handle_action(&WorkspaceAction::OpenNotebook { id: sync_id }, view_ctx);
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "action": ActionKind::DriveNotebookOpen.as_str(),
        "opened": true,
        "id": params.id,
        "window_id": window_id.to_string(),
    }))
}

pub(crate) fn drive_env_var_collection_open(
    target: &TargetSelector,
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_drive_open_target(target, ActionKind::DriveEnvVarCollectionOpen)?;
    let params = action.params_as::<DriveInspectParams>()?;
    let env_var_collection_type = ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
        JsonObjectType::EnvVarCollection,
    ));
    let sync_id = resolve_typed_drive_sync_id(
        &params.id,
        env_var_collection_type,
        ActionKind::DriveEnvVarCollectionOpen,
        ctx,
    )?;
    let window_id =
        select_window_for_drive_open(ActionKind::DriveEnvVarCollectionOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::DriveEnvVarCollectionOpen, window_id, ctx)?;
    workspace.update(ctx, |workspace, view_ctx| {
        workspace.open_env_var_collection(
            &EnvVarCollectionSource::Existing(sync_id),
            false,
            view_ctx,
        );
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "action": ActionKind::DriveEnvVarCollectionOpen.as_str(),
        "opened": true,
        "id": params.id,
        "window_id": window_id.to_string(),
    }))
}

pub(crate) fn drive_object_share_open(
    target: &TargetSelector,
    action: &::local_control::Action,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_drive_open_target(target, ActionKind::DriveObjectShareOpen)?;
    let params = action.params_as::<DriveInspectParams>()?;
    let object_type_and_id =
        resolve_cloud_object_type_and_id(&params.id, ActionKind::DriveObjectShareOpen, ctx)?;
    let window_id = select_window_for_drive_open(ActionKind::DriveObjectShareOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::DriveObjectShareOpen, window_id, ctx)?;
    workspace.update(ctx, |workspace, view_ctx| {
        workspace.handle_action(
            &WorkspaceAction::OpenObjectSharingSettings {
                object_id: object_type_and_id,
                source: SharingDialogSource::DriveIndex,
            },
            view_ctx,
        );
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "action": ActionKind::DriveObjectShareOpen.as_str(),
        "opened": true,
        "id": params.id,
        "object_type": cloud_object_type_label(object_type_and_id),
        "window_id": window_id.to_string(),
    }))
}

fn drive_object_summary(object: &dyn CloudObject) -> Option<DriveObjectSummary> {
    Some(DriveObjectSummary {
        object_type: control_drive_object_type(object)?,
        id: object.uid(),
        name: object.display_name(),
    })
}

fn drive_object_get_result(object: &dyn CloudObject) -> Result<DriveInspectResult, ControlError> {
    let summary = drive_object_summary(object).ok_or_else(|| {
        ControlError::new(
            ErrorCode::UnsupportedAction,
            "drive.inspect does not support this Drive object type",
        )
    })?;
    Ok(DriveInspectResult {
        object: summary,
        content: drive_object_content(object)?,
    })
}

fn control_drive_object_type(object: &dyn CloudObject) -> Option<DriveObjectType> {
    match object.object_type() {
        ObjectType::Workflow => {
            let workflow = object.as_any().downcast_ref::<CloudWorkflow>()?;
            if workflow.model().data.is_agent_mode_workflow() {
                Some(DriveObjectType::Prompt)
            } else {
                Some(DriveObjectType::Workflow)
            }
        }
        ObjectType::Notebook => Some(DriveObjectType::Notebook),
        ObjectType::Folder => Some(DriveObjectType::Folder),
        ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
            JsonObjectType::EnvVarCollection,
        )) => Some(DriveObjectType::EnvVarCollection),
        ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
            JsonObjectType::AIFact,
        )) => Some(DriveObjectType::AiFact),
        ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
            JsonObjectType::MCPServer | JsonObjectType::TemplatableMCPServer,
        )) => Some(DriveObjectType::McpServer),
        _ => None,
    }
}

fn drive_object_content(object: &dyn CloudObject) -> Result<serde_json::Value, ControlError> {
    match control_drive_object_type(object).ok_or_else(drive_unsupported_type_error)? {
        DriveObjectType::Workflow | DriveObjectType::Prompt => object
            .as_any()
            .downcast_ref::<CloudWorkflow>()
            .ok_or_else(drive_type_mismatch_error)
            .and_then(|workflow| {
                serde_json::to_value(&workflow.model().data).map_err(json_response_error)
            }),
        DriveObjectType::Notebook => {
            let notebook = object
                .as_any()
                .downcast_ref::<CloudNotebook>()
                .ok_or_else(drive_type_mismatch_error)?;
            Ok(json!({
                "title": notebook.model().title.clone(),
                "data": notebook.model().data.clone(),
                "ai_document_id": notebook.model().ai_document_id.as_ref().map(|id| id.to_string()),
                "conversation_id": notebook.model().conversation_id.clone(),
            }))
        }
        DriveObjectType::EnvVarCollection => object
            .as_any()
            .downcast_ref::<CloudEnvVarCollection>()
            .ok_or_else(drive_type_mismatch_error)
            .and_then(|env_var_collection| {
                serde_json::to_value(&env_var_collection.model().string_model)
                    .map_err(json_response_error)
            }),
        DriveObjectType::Folder => {
            let folder = object
                .as_any()
                .downcast_ref::<CloudFolder>()
                .ok_or_else(drive_type_mismatch_error)?;
            Ok(json!({
                "name": folder.model().name.clone(),
                "is_open": folder.model().is_open,
                "is_warp_pack": folder.model().is_warp_pack,
            }))
        }
        DriveObjectType::AiFact
        | DriveObjectType::McpServer
        | DriveObjectType::McpServerCollection
        | DriveObjectType::AiRule
        | DriveObjectType::Space
        | DriveObjectType::Trash => Err(drive_unsupported_type_error()),
    }
}

fn drive_object_type_rank(object_type: DriveObjectType) -> u8 {
    match object_type {
        DriveObjectType::Workflow => 0,
        DriveObjectType::Prompt => 1,
        DriveObjectType::Notebook => 2,
        DriveObjectType::EnvVarCollection => 3,
        DriveObjectType::Folder => 4,
        DriveObjectType::AiFact => 5,
        DriveObjectType::McpServer => 6,
        DriveObjectType::McpServerCollection => 7,
        DriveObjectType::AiRule => 8,
        DriveObjectType::Space => 9,
        DriveObjectType::Trash => 10,
    }
}

fn drive_type_mismatch_error() -> ControlError {
    ControlError::new(
        ErrorCode::TargetStateConflict,
        "drive.inspect Drive object type does not match the resolved object",
    )
}

fn drive_unsupported_type_error() -> ControlError {
    ControlError::new(
        ErrorCode::UnsupportedAction,
        "drive.inspect content reads are not supported for this Drive object type",
    )
}

fn json_response_error(error: serde_json::Error) -> ControlError {
    ControlError::with_details(
        ErrorCode::Internal,
        "failed to encode local-control Drive response",
        error.to_string(),
    )
}

fn resolve_cloud_object_type_and_id(
    id: &str,
    action: ActionKind,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<CloudObjectTypeAndId, ControlError> {
    if id.trim().is_empty() {
        return Err(ControlError::new(
            ErrorCode::InvalidParams,
            format!("{} requires a non-empty Drive object id", action.as_str()),
        ));
    }
    let owned_id = id.to_owned();
    let object = CloudModel::as_ref(ctx)
        .get_by_uid(&owned_id)
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::StaleTarget,
                format!(
                    "{} could not resolve the requested Drive object id",
                    action.as_str()
                ),
            )
        })?;
    Ok(object.cloud_object_type_and_id())
}

fn resolve_typed_drive_sync_id(
    id: &str,
    expected: ObjectType,
    action: ActionKind,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<SyncId, ControlError> {
    let object_type_and_id = resolve_cloud_object_type_and_id(id, action, ctx)?;
    if object_type_and_id.object_type() != expected {
        return Err(ControlError::new(
            ErrorCode::TargetStateConflict,
            format!(
                "{} can only open Drive objects of type {}",
                action.as_str(),
                expected
            ),
        ));
    }
    Ok(object_type_and_id.sync_id())
}

fn select_window_for_drive_open(
    action: ActionKind,
    target: &TargetSelector,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<WindowId, ControlError> {
    match target.window.as_ref() {
        None | Some(WindowTarget::Active) => {
            require_active_window_id_for_action(ctx.windows().active_window(), action)
        }
        Some(WindowTarget::Id { id }) => ctx
            .window_ids()
            .find(|window_id| window_id.to_string() == id.0)
            .ok_or_else(|| {
                ControlError::new(
                    ErrorCode::StaleTarget,
                    format!("{} cannot resolve the requested window id", action.as_str()),
                )
            }),
        Some(WindowTarget::Index { .. } | WindowTarget::Title { .. }) => Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} only supports active and opaque window id selectors",
                action.as_str()
            ),
        )),
    }
}

fn workspace_for_window(
    action: ActionKind,
    window_id: WindowId,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<ViewHandle<Workspace>, ControlError> {
    ctx.views_of_type::<Workspace>(window_id)
        .and_then(|workspaces| workspaces.into_iter().next())
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                format!(
                    "{} requires a workspace in the target window",
                    action.as_str()
                ),
            )
        })
}

fn cloud_object_type_label(object_type_and_id: CloudObjectTypeAndId) -> &'static str {
    match object_type_and_id {
        CloudObjectTypeAndId::Notebook(_) => "notebook",
        CloudObjectTypeAndId::Workflow(_) => "workflow",
        CloudObjectTypeAndId::Folder(_) => "folder",
        CloudObjectTypeAndId::GenericStringObject {
            object_type: GenericStringObjectFormat::Json(JsonObjectType::EnvVarCollection),
            ..
        } => "env_var_collection",
        CloudObjectTypeAndId::GenericStringObject {
            object_type: GenericStringObjectFormat::Json(JsonObjectType::AIFact),
            ..
        } => "ai_fact",
        CloudObjectTypeAndId::GenericStringObject {
            object_type:
                GenericStringObjectFormat::Json(
                    JsonObjectType::MCPServer | JsonObjectType::TemplatableMCPServer,
                ),
            ..
        } => "mcp_server",
        CloudObjectTypeAndId::GenericStringObject { .. } => "generic_string_object",
    }
}
