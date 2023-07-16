use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileConfig {
    pub banner: bool,
    pub verbose: bool,
    pub sort: bool,
    pub preview_cmd: Option<FilePreviewCommands>,
    pub preview_width: Option<u32>,
    pub default_dir: String,
    #[serde(rename = "entry")]
    pub entries: Vec<FileEntry>,
}

#[derive(Deserialize, Clone, Serialize, Debug)]
pub struct FilePreviewCommands {
    pub running: Arc<str>,
    pub not_running: Option<Arc<str>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileEntry {
    pub kind: FileEntryKind,
    pub name: String,
    pub workdir: String,
    pub excludes: Option<Vec<String>>,
    pub preview_cmd: Option<FilePreviewCommands>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum FileEntryKind {
    Dir,
    Plain,
}
