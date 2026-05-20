use std::collections::HashMap;

use super::{
    artifact_from_fork_proto, AIConversation, AIConversationAutoexecuteMode, AIConversationId,
};
use crate::ai::artifacts::Artifact;
use crate::persistence::model::AgentConversationData;
use warp_core::features::FeatureFlag;
use warp_multi_agent_api as api;

fn restored_conversation(conversation_data: Option<AgentConversationData>) -> AIConversation {
    AIConversation::new_restored(
        AIConversationId::new(),
        vec![api::Task {
            id: "root-task".to_string(),
            messages: vec![],
            dependencies: None,
            description: String::new(),
            summary: String::new(),
            server_data: String::new(),
        }],
        conversation_data,
    )
    .unwrap()
}

fn user_query_message(id: &str, request_id: &str, query: &str) -> api::Message {
    api::Message {
        id: id.to_string(),
        task_id: "root-task".to_string(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query: query.to_string(),
            context: None,
            referenced_attachments: HashMap::new(),
            mode: None,
            intended_agent: Default::default(),
        })),
        request_id: request_id.to_string(),
        timestamp: None,
    }
}

fn agent_output_message(id: &str, request_id: &str) -> api::Message {
    api::Message {
        id: id.to_string(),
        task_id: "root-task".to_string(),
        server_message_data: String::new(),
        citations: vec![],
        message: Some(api::message::Message::AgentOutput(
            api::message::AgentOutput {
                text: "Done".to_string(),
            },
        )),
        request_id: request_id.to_string(),
        timestamp: None,
    }
}

fn restored_conversation_with_queries(queries: &[&str]) -> AIConversation {
    let messages = queries
        .iter()
        .enumerate()
        .flat_map(|(index, query)| {
            let request_id = format!("request-{index}");
            [
                user_query_message(&format!("user-{index}"), &request_id, query),
                agent_output_message(&format!("agent-{index}"), &request_id),
            ]
        })
        .collect();

    AIConversation::new_restored(
        AIConversationId::new(),
        vec![api::Task {
            id: "root-task".to_string(),
            messages,
            dependencies: None,
            description: String::new(),
            summary: String::new(),
            server_data: String::new(),
        }],
        None,
    )
    .unwrap()
}

#[test]
fn latest_user_query_returns_latest_non_empty_user_query() {
    let conversation =
        restored_conversation_with_queries(&["write unit tests", "fix the failing test"]);

    assert_eq!(
        conversation.latest_user_query(),
        Some("fix the failing test".to_string())
    );
}

#[test]
fn latest_user_query_trims_and_skips_empty_queries() {
    let conversation = restored_conversation_with_queries(&["  write unit tests  ", "  "]);

    assert_eq!(
        conversation.latest_user_query(),
        Some("write unit tests".to_string())
    );
}

#[test]
fn restored_conversation_defaults_autoexecute_override_when_not_persisted() {
    let _flag = FeatureFlag::RememberFastForwardState.override_enabled(true);
    let conversation_data: AgentConversationData =
        serde_json::from_str(r#"{"server_conversation_token":null}"#).unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert_eq!(
        conversation.autoexecute_override(),
        AIConversationAutoexecuteMode::RespectUserSettings
    );
}

#[test]
fn restored_conversation_uses_persisted_last_event_sequence() {
    let conversation_data: AgentConversationData =
        serde_json::from_str(r#"{"server_conversation_token":null,"last_event_sequence":42}"#)
            .unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert_eq!(conversation.last_event_sequence(), Some(42));
}

#[test]
fn restored_conversation_uses_persisted_remote_child_marker() {
    let conversation_data: AgentConversationData =
        serde_json::from_str(r#"{"server_conversation_token":null,"is_remote_child":true}"#)
            .unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert!(conversation.is_remote_child());
}

#[test]
fn child_conversation_detection_uses_parent_agent_id() {
    let conversation_data: AgentConversationData = serde_json::from_str(
        r#"{"server_conversation_token":null,"parent_agent_id":"parent-run-id"}"#,
    )
    .unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert!(conversation.is_child_agent_conversation());
    assert_eq!(conversation.parent_conversation_id(), None);
}

#[test]
fn cli_agent_transcript_vehicle_is_excluded_from_navigation() {
    let conversation = AIConversation::new(false, true);

    assert!(conversation.should_exclude_from_navigation());
}

#[test]
fn restored_conversation_defaults_unknown_persisted_autoexecute_override() {
    let _flag = FeatureFlag::RememberFastForwardState.override_enabled(true);
    let conversation_data: AgentConversationData = serde_json::from_str(
        r#"{"server_conversation_token":null,"autoexecute_override":"UnexpectedValue"}"#,
    )
    .unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert_eq!(
        conversation.autoexecute_override(),
        AIConversationAutoexecuteMode::RespectUserSettings
    );
}

#[test]
fn restored_conversation_uses_persisted_autoexecute_override_when_enabled() {
    let _flag = FeatureFlag::RememberFastForwardState.override_enabled(true);
    let conversation_data: AgentConversationData = serde_json::from_str(
        r#"{"server_conversation_token":null,"autoexecute_override":"RunToCompletion"}"#,
    )
    .unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert_eq!(
        conversation.autoexecute_override(),
        AIConversationAutoexecuteMode::RunToCompletion
    );
}

#[test]
fn restored_conversation_ignores_persisted_autoexecute_override_when_disabled() {
    let _flag = FeatureFlag::RememberFastForwardState.override_enabled(false);
    let conversation_data: AgentConversationData = serde_json::from_str(
        r#"{"server_conversation_token":null,"autoexecute_override":"RunToCompletion"}"#,
    )
    .unwrap();

    let conversation = restored_conversation(Some(conversation_data));

    assert_eq!(
        conversation.autoexecute_override(),
        AIConversationAutoexecuteMode::RespectUserSettings
    );
}

#[test]
fn streamed_byok_custom_slug_survives_conversation_usage_ingestion() {
    const CUSTOM_SLUG: &str = "custom-model-slug";

    let mut conversation = AIConversation::new(false, false);
    let usage_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::new(),
        byok_token_usage: HashMap::from([(
            CUSTOM_SLUG.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 17,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 17)]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(usage_metadata), false)
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 1);
    assert_eq!(token_usage[0].model_id, CUSTOM_SLUG);
    assert_eq!(token_usage[0].byok_tokens, 17);
    assert_eq!(
        token_usage[0]
            .byok_token_usage_by_category
            .get("primary_agent"),
        Some(&17)
    );
}

#[test]
fn mixed_warp_and_byok_usage_survives_conversation_usage_ingestion() {
    const CUSTOM_SLUG: &str = "custom-model-slug";
    const HOSTED_MODEL: &str = "claude-3-5-sonnet";

    let mut conversation = AIConversation::new(false, false);
    let usage_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::from([(
            HOSTED_MODEL.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 42,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 42)]),
                ..Default::default()
            },
        )]),
        byok_token_usage: HashMap::from([(
            CUSTOM_SLUG.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 17,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 17)]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(usage_metadata), false)
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 2);

    let hosted = token_usage
        .iter()
        .find(|u| u.model_id == HOSTED_MODEL)
        .expect("hosted model entry should exist");
    assert_eq!(hosted.warp_tokens, 42);
    assert_eq!(
        hosted
            .warp_token_usage_by_category
            .get("primary_agent"),
        Some(&42)
    );

    let custom = token_usage
        .iter()
        .find(|u| u.model_id == CUSTOM_SLUG)
        .expect("custom slug entry should exist");
    assert_eq!(custom.byok_tokens, 17);
    assert_eq!(
        custom
            .byok_token_usage_by_category
            .get("primary_agent"),
        Some(&17)
    );
}

#[test]
fn byok_custom_slug_with_multiple_categories_survives_ingestion() {
    const CUSTOM_SLUG: &str = "custom-model-slug";

    let mut conversation = AIConversation::new(false, false);
    let usage_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::new(),
        byok_token_usage: HashMap::from([(
            CUSTOM_SLUG.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 25,
                token_usage_by_category: HashMap::from([
                    ("primary_agent".to_string(), 10),
                    ("tool_summarization".to_string(), 15),
                ]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(usage_metadata), false)
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 1);
    assert_eq!(token_usage[0].model_id, CUSTOM_SLUG);
    assert_eq!(token_usage[0].byok_tokens, 25);
    assert_eq!(
        token_usage[0]
            .byok_token_usage_by_category
            .get("primary_agent"),
        Some(&10)
    );
    assert_eq!(
        token_usage[0]
            .byok_token_usage_by_category
            .get("tool_summarization"),
        Some(&15)
    );
}

#[test]
fn multiple_distinct_byok_slugs_remain_separate_in_conversation_usage() {
    const SLUG_A: &str = "custom-model-a";
    const SLUG_B: &str = "custom-model-b";

    let mut conversation = AIConversation::new(false, false);
    let usage_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::new(),
        byok_token_usage: HashMap::from([
            (
                SLUG_A.to_string(),
                api::response_event::stream_finished::ModelTokenUsage {
                    total_tokens: 7,
                    token_usage_by_category: HashMap::from([("primary_agent".to_string(), 7)]),
                    ..Default::default()
                },
            ),
            (
                SLUG_B.to_string(),
                api::response_event::stream_finished::ModelTokenUsage {
                    total_tokens: 13,
                    token_usage_by_category: HashMap::from([("primary_agent".to_string(), 13)]),
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(usage_metadata), false)
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 2);

    let usage_a = token_usage
        .iter()
        .find(|u| u.model_id == SLUG_A)
        .expect("slug A entry should exist");
    assert_eq!(usage_a.byok_tokens, 7);

    let usage_b = token_usage
        .iter()
        .find(|u| u.model_id == SLUG_B)
        .expect("slug B entry should exist");
    assert_eq!(usage_b.byok_tokens, 13);
}

#[test]
fn byok_slug_colliding_with_hosted_model_name_keeps_both_entries() {
    const SHARED_NAME: &str = "claude-3-5-sonnet";

    let mut conversation = AIConversation::new(false, false);
    let usage_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::from([(
            SHARED_NAME.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 30,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 30)]),
                ..Default::default()
            },
        )]),
        byok_token_usage: HashMap::from([(
            SHARED_NAME.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 20,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 20)]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(usage_metadata), false)
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 1);
    assert_eq!(token_usage[0].model_id, SHARED_NAME);
    assert_eq!(token_usage[0].warp_tokens, 30);
    assert_eq!(token_usage[0].byok_tokens, 20);
}

#[test]
fn byok_usage_replaces_on_each_turn_reflecting_latest_server_state() {
    const SLUG: &str = "custom-model-slug";

    let mut conversation = AIConversation::new(false, false);

    let first_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::new(),
        byok_token_usage: HashMap::from([(
            SLUG.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 10,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 10)]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(first_metadata), false)
        .unwrap();

    assert_eq!(conversation.token_usage()[0].byok_tokens, 10);

    let second_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::new(),
        byok_token_usage: HashMap::from([(
            SLUG.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 25,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 25)]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(None, vec![], Some(second_metadata), false)
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 1);
    assert_eq!(token_usage[0].model_id, SLUG);
    assert_eq!(token_usage[0].byok_tokens, 25);
}

#[test]
fn legacy_token_usage_vec_and_byok_metadata_coexist_in_separate_fields() {
    const SLUG: &str = "custom-model-slug";
    const LEGACY_MODEL: &str = "gpt-4o";

    let mut conversation = AIConversation::new(false, false);

    let legacy_token_usage = vec![api::response_event::stream_finished::TokenUsage {
        model_id: LEGACY_MODEL.to_string(),
        total_input: 5,
        output: 3,
        input_cache_read: 0,
        input_cache_write: 0,
        cost_in_cents: 0.0,
    }];

    let usage_metadata = api::response_event::stream_finished::ConversationUsageMetadata {
        context_window_usage: 0.0,
        summarized: false,
        credits_spent: 0.0,
        tool_usage_metadata: None,
        warp_token_usage: HashMap::new(),
        byok_token_usage: HashMap::from([(
            SLUG.to_string(),
            api::response_event::stream_finished::ModelTokenUsage {
                total_tokens: 12,
                token_usage_by_category: HashMap::from([("primary_agent".to_string(), 12)]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    conversation
        .update_cost_and_usage_for_request(
            None,
            legacy_token_usage,
            Some(usage_metadata),
            false,
        )
        .unwrap();

    let token_usage = conversation.token_usage();
    assert_eq!(token_usage.len(), 1);
    assert_eq!(token_usage[0].model_id, SLUG);
    assert_eq!(token_usage[0].byok_tokens, 12);

    let total_usage = &conversation.total_token_usage_by_model;
    assert_eq!(total_usage.len(), 1);
    assert_eq!(
        total_usage
            .iter()
            .find(|u| u.1.model_id == LEGACY_MODEL)
            .map(|u| u.1.total_input),
        Some(5)
    );
}

#[test]
fn fork_artifacts_adds_file_artifacts_to_conversation() {
    let proto_artifact = api::message::artifact_event::ConversationArtifact {
        artifact: Some(
            api::message::artifact_event::conversation_artifact::Artifact::File(
                api::message::artifact_event::FileArtifact {
                    artifact_uid: "artifact-file-1".to_string(),
                    filepath: "outputs/report.txt".to_string(),
                    mime_type: "text/plain".to_string(),
                    size_bytes: 42,
                    description: "Daily summary".to_string(),
                },
            ),
        ),
    };

    assert_eq!(
        artifact_from_fork_proto(&proto_artifact),
        Some(Artifact::File {
            artifact_uid: "artifact-file-1".to_string(),
            filepath: "outputs/report.txt".to_string(),
            filename: "report.txt".to_string(),
            mime_type: "text/plain".to_string(),
            description: Some("Daily summary".to_string()),
            size_bytes: Some(42),
        })
    );
}
