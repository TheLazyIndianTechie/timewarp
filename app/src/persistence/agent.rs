use diesel::associations::HasTable;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sql_types::BigInt;
use diesel::SqliteConnection;
use prost::Message;
use warp_multi_agent_api as api;

use super::model::{AgentConversation, AgentConversationData};
use crate::persistence::model::{
    AgentConversationRecord, AgentConversationSummary, AgentTaskRecord,
};
use crate::persistence::schema::{self, agent_conversations, agent_tasks};

const MAX_STARTUP_TASK_SAMPLE_BYTES: i64 = 256 * 1024;

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = agent_conversations)]
struct NewAgentConversation {
    conversation_id: String,
    conversation_data: String,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = agent_tasks)]
struct NewAgentTask {
    conversation_id: String,
    task_id: String,
    task: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum UpsertConversationError {
    #[error("Failed to serialize conversation data: {0:?}")]
    Serialization(#[from] serde_json::Error),
    #[error("Failed to upsert conversation to sqlite: {0:?}")]
    DB(#[from] diesel::result::Error),
}

pub(super) fn upsert_agent_conversation<'a>(
    conn: &mut SqliteConnection,
    conversation_id_param: &str,
    tasks: impl IntoIterator<Item = &'a api::Task>,
    conversation_data_param: AgentConversationData,
) -> Result<(), UpsertConversationError> {
    use diesel::{ExpressionMethods, QueryDsl};
    use schema::agent_conversations::dsl::*;
    use schema::agent_tasks::dsl as tasks_dsl;
    const MAX_PERSISTED_CONVERSATION_COUNT: i64 = 100;

    let serialized_conversation_data = serde_json::to_string(&conversation_data_param)?;

    conn.transaction::<_, Error, _>(|conn| {
        // Upsert the conversation level metadata
        let new_conversation = NewAgentConversation {
            conversation_id: conversation_id_param.to_owned(),
            conversation_data: serialized_conversation_data,
        };

        diesel::insert_into(agent_conversations::table())
            .values(&new_conversation)
            .on_conflict(conversation_id)
            .do_update()
            .set(&new_conversation)
            .execute(conn)?;

        // Upsert each task
        for task in tasks {
            let task_binary = task.encode_to_vec();
            let new_task = NewAgentTask {
                conversation_id: conversation_id_param.to_owned(),
                task_id: task.id.clone(),
                task: task_binary,
            };

            if let Err(e) = diesel::insert_into(agent_tasks::table)
                .values(&new_task)
                .on_conflict(tasks_dsl::task_id)
                .do_update()
                .set(&new_task)
                .execute(conn)
            {
                log::warn!("Failed to upsert task {e:?}");
                return Err(e);
            }
        }

        // Prune old conversations if we exceed MAX_PERSISTED_CONVERSATION_COUNT conversations
        let conversation_count: i64 = agent_conversations::table().count().get_result(conn)?;
        if conversation_count > MAX_PERSISTED_CONVERSATION_COUNT {
            // Remove the oldest conversations, keeping only the most recent MAX_PERSISTED_CONVERSATION_COUNT
            let conversations_to_remove: Vec<String> = agent_conversations::table()
                .order(last_modified_at.asc())
                .limit(conversation_count - MAX_PERSISTED_CONVERSATION_COUNT)
                .select(conversation_id)
                .load(conn)?;

            delete_agent_conversations(conn, conversations_to_remove)?;
        }

        Ok(())
    })?;

    Ok(())
}

pub(super) fn read_agent_conversation_summaries(
    conn: &mut SqliteConnection,
) -> Result<Vec<AgentConversationSummary>, diesel::result::Error> {
    use schema::agent_conversations::dsl::*;
    use schema::agent_tasks::dsl as tasks_dsl;

    let conversation_records: Vec<AgentConversationRecord> = agent_conversations
        .select(AgentConversationRecord::as_select())
        .load(conn)?;

    let mut summaries = Vec::with_capacity(conversation_records.len());
    for conversation_record in conversation_records {
        let task_count: i64 = agent_tasks::table
            .filter(tasks_dsl::conversation_id.eq(&conversation_record.conversation_id))
            .select(count_star())
            .first(conn)?;

        let sampled_task_records: Vec<AgentTaskRecord> = agent_tasks::table
            .filter(tasks_dsl::conversation_id.eq(&conversation_record.conversation_id))
            .filter(diesel::dsl::sql::<BigInt>("length(task)").le(MAX_STARTUP_TASK_SAMPLE_BYTES))
            .order(tasks_dsl::id.asc())
            .select(AgentTaskRecord::as_select())
            .load(conn)?;

        let sampled_tasks: Vec<_> = sampled_task_records
            .into_iter()
            .filter_map(
                |task_record| match api::Task::decode(&task_record.task[..]) {
                    Ok(task) => Some(task),
                    Err(e) => {
                        log::error!("Failed to decode sampled task protobuf: {e}");
                        None
                    }
                },
            )
            .collect();

        let task_count = task_count.try_into().unwrap_or_default();
        let is_restorable = if sampled_tasks.len() == task_count {
            AgentConversation {
                conversation: conversation_record.clone(),
                tasks: sampled_tasks.clone(),
            }
            .is_restorable()
        } else {
            true
        };

        let sampled_task = sampled_tasks
            .iter()
            .find(|task| task.dependencies.is_none())
            .or_else(|| sampled_tasks.first())
            .cloned();

        summaries.push(AgentConversationSummary {
            conversation: conversation_record,
            task_count,
            sampled_task,
            is_restorable,
        });
    }

    Ok(summaries)
}

/// Read a single agent conversation by its ID, including decoded tasks.
pub(crate) fn read_agent_conversation_by_id(
    conn: &mut SqliteConnection,
    conversation_id_str: &str,
) -> Result<Option<AgentConversation>, diesel::result::Error> {
    use schema::agent_conversations::dsl as convo_dsl;
    use schema::agent_tasks::dsl as tasks_dsl;

    let maybe_record: Option<AgentConversationRecord> = convo_dsl::agent_conversations
        .filter(convo_dsl::conversation_id.eq(conversation_id_str.to_owned()))
        .select(AgentConversationRecord::as_select())
        .first(conn)
        .optional()?;

    let Some(conversation_record) = maybe_record else {
        return Ok(None);
    };

    let task_records: Vec<AgentTaskRecord> = schema::agent_tasks::table
        .filter(tasks_dsl::conversation_id.eq(conversation_id_str))
        .select(AgentTaskRecord::as_select())
        .load(conn)?;

    let mut decoded_tasks = Vec::new();
    let mut failed_to_decode_task = false;
    for task_record in task_records.into_iter() {
        match api::Task::decode(&task_record.task[..]) {
            Ok(task) => decoded_tasks.push(task),
            Err(e) => {
                log::error!("Failed to decode task protobuf: {e}");
                failed_to_decode_task = true;
            }
        }
    }

    if failed_to_decode_task {
        return Ok(None);
    }

    Ok(Some(AgentConversation {
        conversation: conversation_record,
        tasks: decoded_tasks,
    }))
}

pub(super) fn delete_agent_conversations(
    conn: &mut SqliteConnection,
    conversation_ids: Vec<String>,
) -> Result<(), diesel::result::Error> {
    use diesel::{ExpressionMethods, QueryDsl};
    use schema::agent_conversations::dsl::*;
    use schema::agent_tasks::dsl as tasks_dsl;

    conn.transaction::<_, Error, _>(|conn| {
        // Delete tasks for these conversations first (due to foreign key constraint)
        diesel::delete(
            agent_tasks::table.filter(tasks_dsl::conversation_id.eq_any(&conversation_ids)),
        )
        .execute(conn)?;

        // Delete the conversations themselves
        diesel::delete(
            agent_conversations::table().filter(conversation_id.eq_any(&conversation_ids)),
        )
        .execute(conn)?;

        Ok(())
    })?;

    Ok(())
}
