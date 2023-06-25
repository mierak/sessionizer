use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileConfig {
    pub banner: bool,
    pub verbose: bool,
    pub sort: bool,
    pub preview_cmd: Option<String>,
    pub preview_width: Option<u32>,
    pub default_dir: String,
    #[serde(rename = "entry")]
    pub entries: Vec<FileEntry>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileEntry {
    pub kind: FileEntryKind,
    pub name: String,
    pub workdir: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum FileEntryKind {
    Dir,
    Plain,
}
