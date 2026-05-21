//! A singleton model for storing conversations by ID to enable restoration across terminal views.

#[cfg(feature = "local_fs")]
use diesel::SqliteConnection;
use std::collections::HashMap;
#[cfg(feature = "local_fs")]
use std::sync::{Arc, Mutex};

use warpui::{Entity, SingletonEntity};

use crate::ai::agent::conversation::{AIConversation, AIConversationId};
use crate::ai::blocklist::history_model::convert_persisted_conversation_to_ai_conversation_with_metadata;
#[cfg(feature = "local_fs")]
use crate::persistence::agent::read_agent_conversation_by_id;
#[cfg(test)]
use crate::persistence::model::AgentConversation;
use crate::persistence::model::{AgentConversationData, AgentConversationSummary};
#[cfg(feature = "local_fs")]
use crate::persistence::{database_file_path_for_scope, establish_ro_connection, PersistenceScope};

/// Singleton model that holds restored agent conversations on app startup.
///
/// Loading restored conversations into this model is a means of propagating restored data from
/// sqlite (read at startup) to arbitrary consuming locations in the view/model hierarchy without
/// piping it all the way from the root view to the terminal view(s) that require it.
#[derive(Default)]
pub struct RestoredAgentConversations {
    /// Lightweight conversation summaries loaded at startup.
    conversations: HashMap<AIConversationId, AgentConversationSummary>,
    /// Fully materialized conversations used by tests and in-memory handoff paths.
    loaded_conversations: HashMap<AIConversationId, AIConversation>,
    #[cfg(feature = "local_fs")]
    db_connection: Option<Arc<Mutex<SqliteConnection>>>,
}

fn parent_conversation_id_from_data(data: &AgentConversationData) -> Option<AIConversationId> {
    data.parent_conversation_id
        .as_deref()
        .and_then(|id| AIConversationId::try_from(id.to_owned()).ok())
}

fn summary_is_entirely_passive(summary: &AgentConversationSummary) -> bool {
    let Some(task) = summary.sampled_task.as_ref() else {
        return false;
    };

    let mut has_user_query = false;
    let mut has_passive_request = false;
    for message in &task.messages {
        match &message.message {
            Some(warp_multi_agent_api::message::Message::UserQuery(_)) => {
                has_user_query = true;
            }
            Some(warp_multi_agent_api::message::Message::SystemQuery(system_query)) => {
                if matches!(
                    system_query.r#type,
                    Some(warp_multi_agent_api::message::system_query::Type::AutoCodeDiff(_))
                ) {
                    has_passive_request = true;
                }
            }
            _ => {}
        }
    }

    has_passive_request && !has_user_query
}

impl RestoredAgentConversations {
    pub fn new(conversations: Vec<AgentConversationSummary>) -> Self {
        let mut conversations_by_id = HashMap::new();
        for conversation in conversations.into_iter() {
            let conversation_id =
                match AIConversationId::try_from(conversation.conversation.conversation_id.clone())
                {
                    Ok(id) => id,
                    Err(e) => {
                        log::warn!("Failed to convert conversation ID: {e:?}");
                        continue;
                    }
                };
            conversations_by_id.insert(conversation_id, conversation);
        }

        #[cfg(feature = "local_fs")]
        let db_connection = database_file_path_for_scope(&PersistenceScope::App)
            .to_str()
            .and_then(|db_url| {
                establish_ro_connection(db_url)
                    .ok()
                    .map(|conn| Arc::new(Mutex::new(conn)))
            });

        Self {
            conversations: conversations_by_id,
            loaded_conversations: HashMap::new(),
            #[cfg(feature = "local_fs")]
            db_connection,
        }
    }

    #[cfg(test)]
    pub fn new_from_full_conversations(conversations: Vec<AgentConversation>) -> Self {
        let mut store = Self::default();
        for conversation in conversations {
            let conversation_id = conversation.conversation.conversation_id.clone();
            let Some(conversation) =
                convert_persisted_conversation_to_ai_conversation_with_metadata(conversation)
            else {
                log::warn!(
                    "Failed to convert persisted conversation {conversation_id} to AIConversation"
                );
                continue;
            };
            store
                .loaded_conversations
                .insert(conversation.id(), conversation);
        }
        store
    }

    pub fn should_restore_conversation(&self, id: &AIConversationId) -> bool {
        if self.loaded_conversations.contains_key(id) {
            return true;
        }

        self.conversations.get(id).is_some_and(|summary| {
            summary.is_restorable() && summary.has_tasks() && !summary_is_entirely_passive(summary)
        })
    }

    pub fn parent_conversation_id(&self, id: &AIConversationId) -> Option<AIConversationId> {
        self.loaded_conversations
            .get(id)
            .and_then(|conversation| conversation.parent_conversation_id())
            .or_else(|| {
                self.conversations.get(id).and_then(|summary| {
                    summary
                        .conversation_data()
                        .and_then(|data| parent_conversation_id_from_data(&data))
                })
            })
    }

    /// Removes the restored conversation and returns it, if any.
    pub fn take_conversation(&mut self, id: &AIConversationId) -> Option<AIConversation> {
        if let Some(conversation) = self.loaded_conversations.remove(id) {
            self.conversations.remove(id);
            return Some(conversation);
        }

        self.conversations.get(id)?;
        let conversation = self.load_conversation_from_db(id)?;
        self.conversations.remove(id);
        Some(conversation)
    }

    /// Takes and returns AIConversations for the given IDs, sorted by first exchange start time.
    pub fn take_conversations(
        &mut self,
        conversation_ids: &[AIConversationId],
    ) -> Vec<AIConversation> {
        let mut conversations = Vec::new();
        for &conversation_id in conversation_ids {
            if let Some(conversation) = self.take_conversation(&conversation_id) {
                conversations.push(conversation);
            }
        }

        // Sort by first exchange start time (oldest first)
        conversations.sort_by_key(|conversation| {
            conversation
                .first_exchange()
                .map(|exchange| exchange.start_time)
        });
        conversations
    }

    fn load_conversation_from_db(&self, id: &AIConversationId) -> Option<AIConversation> {
        #[cfg(feature = "local_fs")]
        {
            let persisted_conversation = self.db_connection.clone().and_then(|conn| {
                let mut conn = conn.lock().ok()?;
                match read_agent_conversation_by_id(&mut conn, &id.to_string()) {
                    Ok(Some(conversation)) => Some(conversation),
                    Ok(None) => {
                        log::warn!("No AgentConversation found with id {id}");
                        None
                    }
                    Err(e) => {
                        log::warn!("Failed to read AgentConversation {id}: {e:?}");
                        None
                    }
                }
            })?;

            convert_persisted_conversation_to_ai_conversation_with_metadata(persisted_conversation)
        }

        #[cfg(not(feature = "local_fs"))]
        {
            let _ = id;
            None
        }
    }
}

impl Entity for RestoredAgentConversations {
    type Event = ();
}

impl SingletonEntity for RestoredAgentConversations {}

#[cfg(test)]
#[path = "restored_conversations_tests.rs"]
mod tests;
