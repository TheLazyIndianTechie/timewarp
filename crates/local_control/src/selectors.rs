//! Serializable selectors for local-control target families.
use serde::{Deserialize, Serialize};

/// Opaque window identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowSelector(pub String);

/// Opaque tab identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TabSelector(pub String);

/// Opaque pane identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneSelector(pub String);

/// Opaque terminal or agent session identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionSelector(pub String);

/// Opaque terminal block identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlockSelector(pub String);

/// Opaque Warp Drive object identifier supplied by Warp metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DriveObjectId(pub String);

/// User-facing Warp Drive object families in the local-control contract.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveObjectType {
    Workflow,
    Notebook,
    EnvVarCollection,
    Prompt,
    Folder,
    AiFact,
    AiRule,
    McpServer,
    McpServerCollection,
    Space,
    Trash,
}

/// Hierarchical and orthogonal target selectors for a local-control action.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TargetSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<InstanceTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<TabTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<PaneTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<BlockTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<FileTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_object: Option<DriveObjectTarget>,
}

/// Instance-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstanceTarget {
    Active,
    Id { id: String },
    Pid { pid: u32 },
}

/// Window-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowTarget {
    Active,
    Id { id: WindowSelector },
    Index { index: u32 },
    Title { title: String },
}

/// Tab-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TabTarget {
    Active,
    Id { id: TabSelector },
    Index { index: u32 },
    Title { title: String },
}

/// Pane-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PaneTarget {
    Active,
    Id { id: PaneSelector },
    Index { index: u32 },
}

/// Session-level target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionTarget {
    Active,
    Id { id: SessionSelector },
    Index { index: u32 },
}

/// Terminal block target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockTarget {
    Active,
    Id { id: BlockSelector },
    Index { index: u32 },
}

/// File/path intent target. This is app-state only and never grants file-content access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileTarget {
    Path {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        column: Option<u32>,
    },
}

/// Project/workspace target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectTarget {
    Active,
    Id { id: String },
    Path { path: String },
    Name { name: String },
}

/// Warp Drive object target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DriveObjectTarget {
    Id {
        id: DriveObjectId,
    },
    Lookup {
        object_type: DriveObjectType,
        name_or_path: String,
    },
}
