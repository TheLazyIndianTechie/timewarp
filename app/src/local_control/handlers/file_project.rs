//! App-state file, project, and Warp Drive intent handlers for local-control actions.
use std::path::PathBuf;

use ::local_control::protocol::{
    DriveObjectShareOpenParams, DriveObjectType, DriveOpenParams, FileMutationResult,
    FileOpenParams, ProjectOpenParams, TargetSelector, WindowTarget,
};
use ::local_control::{ActionKind, ControlError, ErrorCode};
use serde_json::json;
use warp_util::path::LineAndColumnArg;
use warpui::{ModelContext, SingletonEntity, TypedActionView, WindowId};

use crate::cloud_object::model::persistence::CloudModel;
use crate::cloud_object::{GenericStringObjectFormat, JsonObjectType, ObjectType};
use crate::drive::items::WarpDriveItemId;
use crate::drive::CloudObjectTypeAndId;
use crate::local_control::LocalControlBridge;
use crate::server::ids::{ServerId, SyncId};
use crate::server::telemetry::SharingDialogSource;
use crate::workspace::{Workspace, WorkspaceAction};

pub(crate) fn open_file(
    target: &TargetSelector,
    params: FileOpenParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_window_scoped_target(ActionKind::FileOpen, target)?;
    if params.new_window {
        return Err(ControlError::new(
            ErrorCode::InvalidParams,
            "file.open --new-window is not implemented in this stack layer",
        ));
    }
    let window_id = select_window(ActionKind::FileOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::FileOpen, window_id, ctx)?;
    let path = PathBuf::from(params.path);
    let line_and_column = line_and_column(params.line, params.column)?;
    workspace.update(ctx, |workspace, ctx| {
        workspace.handle_action(
            &WorkspaceAction::OpenFileInNewTab {
                full_path: path.clone(),
                line_and_column,
            },
            ctx,
        );
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!(FileMutationResult {
        path: path.to_string_lossy().into_owned(),
        tab_id: None,
    }))
}

pub(crate) fn open_project(
    target: &TargetSelector,
    params: ProjectOpenParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_window_scoped_target(ActionKind::ProjectOpen, target)?;
    let window_id = select_window(ActionKind::ProjectOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::ProjectOpen, window_id, ctx)?;
    let path = params.path;
    workspace.update(ctx, |workspace, ctx| {
        workspace.handle_action(
            &WorkspaceAction::OpenRepository {
                path: Some(path.clone()),
            },
            ctx,
        );
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "path": path,
        "window_id": window_id.to_string(),
    }))
}

pub(crate) fn open_drive_object(
    target: &TargetSelector,
    params: DriveOpenParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_window_scoped_target(ActionKind::DriveOpen, target)?;
    let window_id = select_window(ActionKind::DriveOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::DriveOpen, window_id, ctx)?;
    let object_id = drive_object_type_and_id(params.object_type, &params.id);
    let action = match params.object_type {
        DriveObjectType::Notebook => WorkspaceAction::OpenNotebook {
            id: object_id.sync_id(),
        },
        DriveObjectType::Workflow | DriveObjectType::Environment | DriveObjectType::Prompt => {
            WorkspaceAction::ViewObjectInWarpDrive(WarpDriveItemId::Object(object_id))
        }
    };
    workspace.update(ctx, |workspace, ctx| {
        workspace.handle_action(&action, ctx);
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "object_type": params.object_type,
        "id": params.id,
        "window_id": window_id.to_string(),
    }))
}

pub(crate) fn open_drive_object_share_dialog(
    target: &TargetSelector,
    params: DriveObjectShareOpenParams,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    validate_window_scoped_target(ActionKind::DriveObjectShareOpen, target)?;
    let window_id = select_window(ActionKind::DriveObjectShareOpen, target, ctx)?;
    let workspace = workspace_for_window(ActionKind::DriveObjectShareOpen, window_id, ctx)?;
    let server_id = ServerId::from_string_lossy(&params.id);
    let object = CloudModel::as_ref(ctx)
        .get_by_uid(&server_id.uid())
        .map(|object| object.cloud_object_type_and_id())
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                "drive.object.share.open requires a loaded Warp Drive object",
            )
        })?;
    workspace.update(ctx, |workspace, ctx| {
        workspace.handle_action(
            &WorkspaceAction::OpenObjectSharingSettings {
                object_id: object,
                source: SharingDialogSource::CommandPalette,
            },
            ctx,
        );
    });
    ctx.windows().show_window_and_focus_app(window_id);
    Ok(json!({
        "id": params.id,
        "window_id": window_id.to_string(),
    }))
}

fn line_and_column(
    line: Option<u32>,
    column: Option<u32>,
) -> Result<Option<LineAndColumnArg>, ControlError> {
    line.map(|line| {
        let line_num = usize::try_from(line).map_err(|err| {
            ControlError::with_details(
                ErrorCode::InvalidParams,
                "file.open line is too large",
                err.to_string(),
            )
        })?;
        let column_num = column
            .map(|column| {
                usize::try_from(column).map_err(|err| {
                    ControlError::with_details(
                        ErrorCode::InvalidParams,
                        "file.open column is too large",
                        err.to_string(),
                    )
                })
            })
            .transpose()?;
        Ok(LineAndColumnArg {
            line_num,
            column_num,
        })
    })
    .transpose()
}

fn validate_window_scoped_target(
    action: ActionKind,
    target: &TargetSelector,
) -> Result<(), ControlError> {
    if target.tab.is_some()
        || target.pane.is_some()
        || target.session.is_some()
        || target.block.is_some()
        || target.file.is_some()
        || target.drive.is_some()
    {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            format!(
                "{} only accepts instance and window selectors",
                action.as_str()
            ),
        ));
    }
    Ok(())
}

fn select_window(
    action: ActionKind,
    target: &TargetSelector,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<WindowId, ControlError> {
    match target.window.as_ref() {
        None | Some(WindowTarget::Active) => ctx.windows().active_window().ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                format!("{} requires an active Warp window", action.as_str()),
            )
        }),
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
) -> Result<warpui::ViewHandle<Workspace>, ControlError> {
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

fn drive_object_type_and_id(object_type: DriveObjectType, id: &str) -> CloudObjectTypeAndId {
    let sync_id = SyncId::ServerId(ServerId::from_string_lossy(id));
    match object_type {
        DriveObjectType::Workflow | DriveObjectType::Prompt => {
            CloudObjectTypeAndId::Workflow(sync_id)
        }
        DriveObjectType::Notebook => {
            CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Notebook)
        }
        DriveObjectType::Environment => CloudObjectTypeAndId::from_generic_string_object(
            GenericStringObjectFormat::Json(JsonObjectType::EnvVarCollection),
            sync_id,
        ),
    }
}
