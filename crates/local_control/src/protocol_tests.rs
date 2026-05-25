use super::*;

#[test]
fn request_envelope_serializes_stable_action_names() {
    let request = RequestEnvelope::new(Action::new(ActionKind::WindowFocus));
    let value = serde_json::to_value(&request).expect("request serializes");
    assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(value["action"]["kind"], "window.focus");
}

#[test]
fn response_error_serializes_machine_code() {
    let response = ResponseEnvelope::error(
        Uuid::nil(),
        ControlError::new(ErrorCode::UnauthorizedLocalClient, "bad token"),
    );
    let value = serde_json::to_value(&response).expect("response serializes");
    assert_eq!(value["response"]["status"], "error");
    assert_eq!(
        value["response"]["error"]["code"],
        "unauthorized_local_client"
    );
}

#[test]
fn ambiguous_target_error_code_is_stable() {
    let value = serde_json::to_value(ErrorCode::AmbiguousTarget).expect("code serializes");
    assert_eq!(value, serde_json::json!("ambiguous_target"));
}

#[test]
fn input_run_is_cataloged_as_authenticated_underlying_mutation_stub() {
    let action = serde_json::from_value::<ActionKind>(serde_json::json!("input.run"))
        .expect("input.run is cataloged");
    let metadata = action.metadata();
    assert_eq!(
        metadata.implementation_status,
        ActionImplementationStatus::Stub
    );
    assert_eq!(
        metadata.permission_category,
        PermissionCategory::MutateUnderlyingData
    );
    assert_eq!(
        metadata.state_data_category,
        StateDataCategory::UnderlyingDataMutation
    );
    assert!(metadata.requires_authenticated_user);
}

#[test]
fn malformed_action_name_is_not_deserialized() {
    let action = serde_json::from_value::<ActionKind>(serde_json::json!("tab.create.extra"));
    assert!(action.is_err());
}

#[test]
fn tab_create_metadata_is_first_slice_logged_out_safe_mutation() {
    let metadata = ActionKind::TabCreate.metadata();
    assert_eq!(
        metadata.implementation_status,
        ActionImplementationStatus::Implemented
    );
    assert_eq!(metadata.risk_tier, RiskTier::MutatingNonDestructive);
    assert_eq!(
        metadata.state_data_category,
        StateDataCategory::AppStateMutation
    );
    assert!(!metadata.requires_authenticated_user);
    assert!(!metadata.authenticated_user.required);
    assert_eq!(
        metadata.permission_category,
        PermissionCategory::MutateAppState
    );
    assert_eq!(
        metadata.allowed_invocation_contexts,
        vec![
            InvocationContext::InsideWarp,
            InvocationContext::OutsideWarp
        ]
    );
}

#[test]
fn structural_metadata_actions_are_logged_out_safe_read_metadata() {
    for action in [
        ActionKind::InstanceList,
        ActionKind::InstanceInspect,
        ActionKind::AppPing,
        ActionKind::AppInspect,
        ActionKind::AppVersion,
        ActionKind::AppActive,
        ActionKind::ActionList,
        ActionKind::ActionGet,
        ActionKind::CapabilityList,
        ActionKind::CapabilityInspect,
        ActionKind::WindowList,
        ActionKind::WindowInspect,
        ActionKind::TabList,
        ActionKind::TabInspect,
        ActionKind::PaneList,
        ActionKind::PaneInspect,
        ActionKind::SessionList,
        ActionKind::SessionInspect,
    ] {
        let metadata = action.metadata();
        assert_eq!(
            metadata.implementation_status,
            ActionImplementationStatus::Implemented
        );
        assert_eq!(metadata.risk_tier, RiskTier::ReadOnlyMetadata);
        assert_eq!(
            metadata.state_data_category,
            StateDataCategory::MetadataRead
        );
        assert_eq!(
            metadata.permission_category,
            PermissionCategory::ReadMetadata
        );
        assert!(!metadata.requires_authenticated_user);
        assert!(!metadata.authenticated_user.required);
        assert_eq!(
            metadata.allowed_invocation_contexts,
            vec![InvocationContext::OutsideWarp]
        );
    }
}

#[test]
fn structural_metadata_actions_have_expected_target_scopes() {
    for action in [
        ActionKind::InstanceList,
        ActionKind::InstanceInspect,
        ActionKind::AppPing,
        ActionKind::AppInspect,
        ActionKind::AppVersion,
        ActionKind::AppActive,
    ] {
        assert_eq!(action.metadata().target_scope, TargetScope::Instance);
    }

    assert_eq!(
        ActionKind::ActionList.metadata().target_scope,
        TargetScope::Action
    );
    assert_eq!(
        ActionKind::ActionGet.metadata().target_scope,
        TargetScope::Action
    );
    assert_eq!(
        ActionKind::CapabilityList.metadata().target_scope,
        TargetScope::Action
    );
    assert_eq!(
        ActionKind::CapabilityInspect.metadata().target_scope,
        TargetScope::Action
    );
    assert_eq!(
        ActionKind::WindowList.metadata().target_scope,
        TargetScope::Window
    );
    assert_eq!(
        ActionKind::WindowInspect.metadata().target_scope,
        TargetScope::Window
    );
    assert_eq!(
        ActionKind::TabList.metadata().target_scope,
        TargetScope::Tab
    );
    assert_eq!(
        ActionKind::TabInspect.metadata().target_scope,
        TargetScope::Tab
    );
    assert_eq!(
        ActionKind::PaneList.metadata().target_scope,
        TargetScope::Pane
    );
    assert_eq!(
        ActionKind::PaneInspect.metadata().target_scope,
        TargetScope::Pane
    );
    assert_eq!(
        ActionKind::SessionList.metadata().target_scope,
        TargetScope::Session
    );
    assert_eq!(
        ActionKind::SessionInspect.metadata().target_scope,
        TargetScope::Session
    );
}

#[test]
fn underlying_data_actions_require_underlying_data_permission_and_authenticated_user() {
    for action in [
        ActionKind::BlockList,
        ActionKind::BlockGet,
        ActionKind::InputGet,
        ActionKind::HistoryList,
    ] {
        let metadata = action.metadata();
        assert_eq!(
            metadata.implementation_status,
            ActionImplementationStatus::Implemented
        );
        assert_eq!(metadata.risk_tier, RiskTier::ReadOnlyTerminalData);
        assert_eq!(
            metadata.state_data_category,
            StateDataCategory::UnderlyingDataRead
        );
        assert_eq!(
            metadata.permission_category,
            PermissionCategory::ReadUnderlyingData
        );
        assert!(metadata.requires_authenticated_user);
        assert!(metadata.authenticated_user.required);
        assert_eq!(
            metadata.allowed_invocation_contexts,
            vec![
                InvocationContext::InsideWarp,
                InvocationContext::OutsideWarp
            ]
        );
    }
}

#[test]
fn underlying_data_actions_have_expected_target_scopes() {
    assert_eq!(
        ActionKind::BlockList.metadata().target_scope,
        TargetScope::Block
    );
    assert_eq!(
        ActionKind::BlockGet.metadata().target_scope,
        TargetScope::Block
    );
    assert_eq!(
        ActionKind::InputGet.metadata().target_scope,
        TargetScope::Session
    );
    assert_eq!(
        ActionKind::HistoryList.metadata().target_scope,
        TargetScope::History
    );
}

#[test]
fn action_with_params_roundtrips_typed_action_get_params() {
    let action = Action::with_params(
        ActionKind::ActionGet,
        ActionGetParams {
            action: "tab.create".to_owned(),
        },
    )
    .expect("params serialize");
    assert_eq!(action.kind, ActionKind::ActionGet);
    assert_eq!(action.params["action"], "tab.create");

    let params = action
        .params_as::<ActionGetParams>()
        .expect("params deserialize");
    assert_eq!(params.action, "tab.create");
}

#[test]
fn action_metadata_serializes_security_categories() {
    let metadata = ActionKind::TabCreate.metadata();
    let value = serde_json::to_value(metadata).expect("metadata serializes");
    assert_eq!(value["name"], "tab.create");
    assert_eq!(value["state_data_category"], "app_state_mutation");
    assert_eq!(value["permission_category"], "mutate_app_state");
    assert_eq!(
        value["authenticated_user"]["required"],
        serde_json::json!(false)
    );
}

#[test]
fn default_permissions_preserve_security_categories() {
    assert_eq!(
        ActionKind::TabCreate.metadata().permission_category,
        PermissionCategory::MutateAppState
    );
    assert_eq!(
        ActionKind::InputInsert.metadata().permission_category,
        PermissionCategory::MutateAppState
    );
    assert_eq!(
        ActionKind::SettingSet.metadata().permission_category,
        PermissionCategory::MutateMetadataConfiguration
    );
    assert_eq!(
        ActionKind::TabList.metadata().permission_category,
        PermissionCategory::ReadMetadata
    );
    assert_eq!(
        ActionKind::BlockList.metadata().permission_category,
        PermissionCategory::ReadUnderlyingData
    );
}

#[test]
fn non_first_slice_actions_are_catalog_stubs() {
    let metadata = ActionKind::WindowCreate.metadata();
    assert_eq!(
        metadata.implementation_status,
        ActionImplementationStatus::Stub
    );
    assert!(
        metadata
            .allowed_invocation_contexts
            .contains(&InvocationContext::OutsideWarp)
    );
}

#[test]
fn file_content_actions_are_explicitly_excluded() {
    for action_name in EXCLUDED_FILE_CONTENT_ACTION_NAMES {
        let action = serde_json::from_str::<ActionKind>(&format!("\"{action_name}\""));
        assert!(action.is_err(), "{action_name} must not be allowlisted");
    }
    assert_eq!(
        ActionKind::FileOpen.metadata().permission_category,
        PermissionCategory::MutateAppState
    );
    assert_eq!(
        ActionKind::FileList.metadata().permission_category,
        PermissionCategory::ReadMetadata
    );
}

#[test]
fn drive_sharing_contract_distinguishes_dialog_from_team_mutation() {
    let share_open = ActionKind::DriveObjectShareOpen.metadata();
    assert_eq!(share_open.name, "drive.object.share.open");
    assert_eq!(
        share_open.state_data_category,
        StateDataCategory::AppStateMutation
    );
    assert_eq!(
        share_open.permission_category,
        PermissionCategory::MutateAppState
    );
    assert!(share_open.requires_authenticated_user);

    let share_to_team = ActionKind::DriveObjectShareToTeam.metadata();
    assert_eq!(share_to_team.name, "drive.object.share_to_team");
    assert_eq!(
        share_to_team.state_data_category,
        StateDataCategory::UnderlyingDataMutation
    );
    assert_eq!(
        share_to_team.permission_category,
        PermissionCategory::MutateUnderlyingData
    );
    assert!(share_to_team.requires_authenticated_user);
}

#[test]
fn action_catalog_has_unique_stable_names() {
    let mut names = std::collections::BTreeSet::new();
    for action in ActionKind::ALL {
        assert!(
            names.insert(action.as_str()),
            "duplicate action name {}",
            action.as_str()
        );
        let serialized = serde_json::to_value(action).expect("action serializes");
        assert_eq!(serialized, serde_json::json!(action.as_str()));
    }
}

#[test]
fn drive_selector_and_typed_params_serialize_stably() {
    let target = TargetSelector {
        drive_object: Some(DriveObjectTarget::Lookup {
            object_type: DriveObjectType::Notebook,
            name_or_path: "Team/Runbook".to_owned(),
        }),
        ..TargetSelector::default()
    };
    let value = serde_json::to_value(target).expect("target serializes");
    assert_eq!(value["drive_object"]["type"], "lookup");
    assert_eq!(value["drive_object"]["object_type"], "notebook");

    let params = ActionParams::DriveObjectId {
        id: DriveObjectId("drive_123".to_owned()),
    };
    let value = serde_json::to_value(params).expect("params serialize");
    assert_eq!(value["type"], "drive_object_id");
    assert_eq!(value["id"], "drive_123");
}
