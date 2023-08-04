use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileConfig {
    #[serde(default = "default_bool_false")]
    pub no_banner: bool,
    #[serde(default = "default_bool_false")]
    pub verbose: bool,
    #[serde(default = "default_bool_true")]
    pub sort: bool,
    pub preview_cmd: Option<FilePreviewCommands>,
    #[serde(default = "default_preview_width")]
    pub preview_width: u32,
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

impl Default for FileConfig {
    fn default() -> Self {
        FileConfig {
            default_dir: "/".to_owned(),
            no_banner: true,
            verbose: false,
            sort: true,
            preview_cmd: Some(FilePreviewCommands {
                running:
                    Arc::from("tmux capture-pane -pe -t $(tmux list-panes -F '#{pane_id}' -s -t '{{name}}' -f '#{window_active}')".to_owned()),
                not_running: Some(Arc::from("ls -la".to_owned())),
            }),
            preview_width: 30,
            entries: vec![
                FileEntry {
                    name: "My session".to_owned(),
                    workdir: "/".to_owned(),
                    kind: FileEntryKind::Plain,
                    preview_cmd: Some(FilePreviewCommands {
                        running: Arc::from("ls -la".to_owned()),
                        not_running: Some(Arc::from("ls -la".to_owned())),
                    }),
                    excludes: None,
                },
                FileEntry {
                    name: "My Projects Dir - {{name}} {{workdir}}".to_owned(),
                    workdir: "/home/youruser".to_owned(),
                    kind: FileEntryKind::Dir,
                    preview_cmd: None,
                    excludes: Some(vec!["somedir".to_owned()]),
                },
            ],
        }
    }
}

// Default values for serde optional fields
const fn default_bool_false() -> bool {
    false
}
const fn default_bool_true() -> bool {
    true
}
const fn default_preview_width() -> u32 {
    40
}
