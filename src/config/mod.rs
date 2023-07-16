mod args;
mod file_config;

use std::{path::PathBuf, sync::Arc};

use crate::config::{
    args::Args,
    file_config::{FileConfig, FileEntryKind},
};
use anyhow::Result;
use anyhow::{anyhow, Context};
use clap::Parser;
use file_config::FileEntry;

pub use args::Command;
pub use file_config::FilePreviewCommands;

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewCommands {
    pub running: Option<Arc<str>>,
    pub not_running: Option<Arc<str>>,
}

impl From<FilePreviewCommands> for PreviewCommands {
    fn from(value: FilePreviewCommands) -> Self {
        Self {
            running: Some(value.running),
            not_running: value.not_running,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Workdir(std::sync::Arc<str>);

#[derive(Debug)]
pub enum Entry {
    Dir(EntryDir),
    Plain(EntryPlain),
}

#[derive(Debug)]
pub struct EntryDir {
    pub name: String,
    pub workdir: Workdir,
    pub excludes: Option<Vec<String>>,
    pub preview_cmd: Option<PreviewCommands>,
}
#[derive(Debug)]
pub struct EntryPlain {
    pub name: String,
    pub workdir: Workdir,
    pub preview_cmd: Option<PreviewCommands>,
}

impl TryFrom<String> for Workdir {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(crate::utils::envsubst(&value)?))
    }
}

impl AsRef<str> for Workdir {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct Config {
    pub config_path: PathBuf,
    pub command: Option<Command>,
    pub hide_banner: bool,
    pub verbose: bool,
    pub sort: bool,
    pub preview_commands: Option<PreviewCommands>,
    pub preview_width: Option<u32>,
    pub default_dir: Workdir,
    pub dry_run: bool,
}

pub struct ConfigWithEntries(Config, Vec<Entry>);

impl ConfigWithEntries {
    pub fn as_ref(&self) -> (&Config, &Vec<Entry>) {
        (&self.0, &self.1)
    }

    pub fn value(self) -> (Config, Vec<Entry>) {
        (self.0, self.1)
    }
}

struct MaybePreviewCommands(Option<PreviewCommands>);
impl From<(Option<&Arc<str>>, Option<&Arc<str>>, Option<FilePreviewCommands>)> for MaybePreviewCommands {
    fn from(value: (Option<&Arc<str>>, Option<&Arc<str>>, Option<FilePreviewCommands>)) -> Self {
        let res = match (value.0, value.1, value.2) {
            (None, None, Some(v)) => Some(v.into()),
            (Some(r), None, Some(mut v)) => {
                v.running = r.clone();
                Some(PreviewCommands::from(v))
            }
            (None, Some(n), Some(mut v)) => {
                v.not_running = Some(n.clone());
                Some(PreviewCommands::from(v))
            }
            (None, Some(n), None) => Some(PreviewCommands {
                running: None,
                not_running: Some(n.clone()),
            }),
            (Some(r), None, None) => Some(PreviewCommands {
                running: Some(r.clone()),
                not_running: None,
            }),
            (Some(r), Some(n), None) => Some(PreviewCommands {
                running: Some(r.clone()),
                not_running: Some(n.clone()),
            }),
            (Some(r), Some(n), Some(_)) => Some(PreviewCommands {
                running: Some(r.clone()),
                not_running: Some(n.clone()),
            }),
            (None, None, None) => None,
        };
        return Self(res);
    }
}

impl From<(Option<&PreviewCommands>, Option<FilePreviewCommands>)> for MaybePreviewCommands {
    fn from(value: (Option<&PreviewCommands>, Option<FilePreviewCommands>)) -> Self {
        let res = match value {
            (None, Some(v)) => Some(v.into()),
            (Some(v), None) => Some(v.to_owned()),
            (Some(default), Some(specific)) => Some(PreviewCommands {
                running: Some(specific.running),
                not_running: specific.not_running.or(default.not_running.as_ref().cloned()),
            }),
            (None, None) => None,
        };
        return Self(res);
    }
}

impl Config {
    pub fn read() -> Result<ConfigWithEntries> {
        let mut args = Args::parse();
        let file_config: FileConfig = toml::from_str(
            &std::fs::read_to_string(&args.config)
                .context(format!("Unable to read config file '{:?}'", &args.config))?,
        )?;

        let preview = args.preview.take().map(Arc::from);
        let preview_no_session = args.preview_no_session.take().map(Arc::from);
        let preview_commands =
            MaybePreviewCommands::from((preview.as_ref(), preview_no_session.as_ref(), file_config.preview_cmd)).0;

        let mut entries = Vec::with_capacity(file_config.entries.len());
        for ele in file_config.entries {
            let res = match ele.kind {
                FileEntryKind::Dir => Entry::Dir(EntryDir {
                    name: ele.name,
                    workdir: ele.workdir.try_into()?,
                    excludes: ele.excludes,
                    preview_cmd: MaybePreviewCommands::from((preview_commands.as_ref(), ele.preview_cmd)).0,
                }),
                FileEntryKind::Plain => {
                    if ele.excludes.is_some() {
                        return Err(anyhow!(
                            "Entry '{}' is invalid. Excludes are not allowed on 'Plain' entries.",
                            ele.name
                        ));
                    }
                    Entry::Plain(EntryPlain {
                        name: ele.name,
                        workdir: ele.workdir.try_into()?,
                        preview_cmd: MaybePreviewCommands::from((preview_commands.as_ref(), ele.preview_cmd)).0,
                    })
                }
            };
            entries.push(res);
        }

        return Ok(ConfigWithEntries(
            Config {
                preview_commands,
                config_path: args.config,
                command: args.command.to_owned(),
                hide_banner: args.no_banner || !file_config.banner,
                verbose: args.verbose || file_config.verbose,
                sort: args.sort || file_config.sort,
                preview_width: file_config.preview_width,
                dry_run: args.dry_run,
                default_dir: file_config.default_dir.try_into()?,
            },
            entries,
        ));
    }

    pub fn get_dummy_config_file() -> Result<String> {
        let home = std::env::var("HOME").context("HOME env variable not set")?;
        toml::to_string(&FileConfig {
            default_dir: "/".to_owned(),
            banner: true,
            verbose: false,
            sort: true,
            preview_cmd: Some(FilePreviewCommands {
                running:
                    Arc::from("tmux capture-pane -pe -t $(tmux list-panes -F '#{pane_id}' -s -t '{{name}}' -f '#{window_active}')".to_owned()),
                not_running: Some(Arc::from("ls -la".to_owned())),
            }),
            preview_width: Some(30),
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
                    name: "My Projects Dir".to_owned(),
                    workdir: home,
                    kind: FileEntryKind::Dir,
                    preview_cmd: None,
                    excludes: Some(vec!["somedir".to_owned()]),
                },
            ],
        })
        .context("Unable to serialize default config")
    }
}
