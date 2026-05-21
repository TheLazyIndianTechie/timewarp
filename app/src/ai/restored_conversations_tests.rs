use chrono::NaiveDateTime;

use super::RestoredAgentConversations;
use crate::ai::agent::conversation::AIConversationId;
use crate::persistence::model::{
    AgentConversationData, AgentConversationRecord, AgentConversationSummary,
};

fn conversation_data(parent_conversation_id: Option<AIConversationId>) -> AgentConversationData {
    AgentConversationData {
        server_conversation_token: None,
        conversation_usage_metadata: None,
        reverted_action_ids: None,
        forked_from_server_conversation_token: None,
        artifacts_json: None,
        parent_agent_id: None,
        agent_name: None,
        orchestration_harness_type: None,
        parent_conversation_id: parent_conversation_id.map(|id| id.to_string()),
        is_remote_child: false,
        root_task_is_optimistic: None,
        run_id: None,
        autoexecute_override: None,
        last_event_sequence: None,
        pinned: false,
    }
}

fn conversation_summary(
    conversation_id: AIConversationId,
    task_count: usize,
    parent_conversation_id: Option<AIConversationId>,
) -> AgentConversationSummary {
    AgentConversationSummary {
        conversation: AgentConversationRecord {
            id: 0,
            conversation_id: conversation_id.to_string(),
            conversation_data: serde_json::to_string(&conversation_data(parent_conversation_id))
                .expect("conversation data should serialize"),
            last_modified_at: NaiveDateTime::default(),
        },
        task_count,
        sampled_task: None,
        is_restorable: true,
    }
}

#[test]
fn summary_store_restores_task_containing_conversations_without_sampled_task() {
    let conversation_id = AIConversationId::new();
    let parent_id = AIConversationId::new();
    let empty_conversation_id = AIConversationId::new();
    let store = RestoredAgentConversations::new(vec![
        conversation_summary(conversation_id, 2, Some(parent_id)),
        conversation_summary(empty_conversation_id, 0, None),
    ]);

    assert!(
        store.should_restore_conversation(&conversation_id),
        "task count metadata is enough to mark an unsampled conversation restorable",
    );
    assert_eq!(
        store.parent_conversation_id(&conversation_id),
        Some(parent_id),
        "parent metadata should be available without loading tasks",
    );
    assert!(
        !store.should_restore_conversation(&empty_conversation_id),
        "conversations with no persisted tasks should not be restored",
    );
}
