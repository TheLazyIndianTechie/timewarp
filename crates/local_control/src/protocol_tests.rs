use super::*;

fn action_name(kind: ActionKind) -> String {
    serde_json::to_value(kind)
        .expect("action kind serializes")
        .as_str()
        .expect("action kind serializes as string")
        .to_owned()
}

fn error_code_name(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .expect("error code serializes")
        .as_str()
        .expect("error code serializes as string")
        .to_owned()
}

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
fn input_run_is_not_in_the_allowlisted_catalog() {
    let action = serde_json::from_value::<ActionKind>(serde_json::json!("input.run"));
    assert!(action.is_err());
}

#[test]
fn file_content_commands_are_not_in_the_allowlisted_catalog() {
    for action in [
        "file.read",
        "file.write",
        "file.append",
        "file.delete",
        "drive.object.share-public",
        "drive.object.share-external",
        "agent.prompt.submit",
    ] {
        assert!(serde_json::from_value::<ActionKind>(serde_json::json!(action)).is_err());
    }
}
#[test]
fn malformed_action_name_is_not_deserialized() {
    let action = serde_json::from_value::<ActionKind>(serde_json::json!("tab.create.extra"));
    assert!(action.is_err());
}

#[test]
fn action_catalog_has_unique_stable_names() {
    let names = ActionKind::ALL
        .iter()
        .copied()
        .map(action_name)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(), ActionKind::ALL.len());

    for action in ActionKind::ALL {
        assert_eq!(action.as_str(), action_name(*action));
        assert_eq!(action.metadata().name, action.as_str());
    }
}

#[test]
fn implemented_metadata_is_exactly_first_slice_actions() {
    let implemented = ActionKind::implemented_metadata()
        .into_iter()
        .map(|metadata| metadata.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        implemented,
        vec![
            ActionKind::InstanceList,
            ActionKind::AppPing,
            ActionKind::AppVersion,
            ActionKind::TabCreate,
        ]
    );
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
        vec![InvocationContext::OutsideWarp]
    );
}

#[test]
fn core_smoke_metadata_has_explicit_read_metadata_category() {
    for action in [
        ActionKind::InstanceList,
        ActionKind::AppPing,
        ActionKind::AppVersion,
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
        assert!(!metadata.authenticated_user.required);
        assert_eq!(metadata.target_scope, TargetScope::Instance);
    }
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
        PermissionCategory::MutateUnderlyingData
    );
    assert_eq!(
        ActionKind::SettingSet.metadata().permission_category,
        PermissionCategory::MutateMetadataConfiguration
    );
    assert_eq!(
        ActionKind::TabList.metadata().permission_category,
        PermissionCategory::ReadMetadata
    );
}

#[test]
fn implemented_actions_have_complete_logged_out_metadata() {
    for metadata in ActionKind::implemented_metadata() {
        assert_eq!(
            metadata.implementation_status,
            ActionImplementationStatus::Implemented
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
fn stub_actions_default_to_authenticated_user_gate_and_no_contexts() {
    for action in ActionKind::ALL.iter().copied().filter(|action| {
        action.metadata().implementation_status == ActionImplementationStatus::Stub
    }) {
        let metadata = action.metadata();
        assert!(metadata.requires_authenticated_user);
        assert!(metadata.authenticated_user.required);
        assert!(metadata.allowed_invocation_contexts.is_empty());
    }
}

#[test]
fn structured_error_codes_have_unique_stable_strings() {
    let codes = [
        ErrorCode::LocalControlDisabled,
        ErrorCode::UnauthorizedLocalClient,
        ErrorCode::InsufficientPermissions,
        ErrorCode::AuthenticatedUserRequired,
        ErrorCode::AuthenticatedUserUnavailable,
        ErrorCode::ExecutionContextNotAllowed,
        ErrorCode::ProtocolVersionUnsupported,
        ErrorCode::InvalidRequest,
        ErrorCode::InvalidSelector,
        ErrorCode::InvalidParams,
        ErrorCode::NoInstance,
        ErrorCode::AmbiguousInstance,
        ErrorCode::AmbiguousTarget,
        ErrorCode::StaleTarget,
        ErrorCode::TargetStateConflict,
        ErrorCode::MissingTarget,
        ErrorCode::TransportUnavailable,
        ErrorCode::BridgeUnavailable,
        ErrorCode::UnsupportedAction,
        ErrorCode::NotAllowlisted,
        ErrorCode::Internal,
    ];
    let serialized = codes
        .into_iter()
        .map(error_code_name)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(serialized.len(), codes.len());
    assert!(serialized.contains("authenticated_user_unavailable"));
    assert!(serialized.contains("not_allowlisted"));
}
#[test]
fn non_first_slice_actions_are_catalog_stubs() {
    let metadata = ActionKind::WindowCreate.metadata();
    assert_eq!(
        metadata.implementation_status,
        ActionImplementationStatus::Stub
    );
    assert!(
        !metadata
            .allowed_invocation_contexts
            .contains(&InvocationContext::OutsideWarp)
    );
}
