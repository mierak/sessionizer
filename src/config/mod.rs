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

#[derive(Debug, PartialEq)]
pub enum Entry {
    Dir(EntryDir),
    Plain(EntryPlain),
}

#[derive(Debug, PartialEq)]
pub struct EntryDir {
    pub name: String,
    pub workdir: Workdir,
    pub excludes: Option<Vec<String>>,
    pub preview_cmd: Option<PreviewCommands>,
}
#[derive(Debug, PartialEq)]
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
    pub preview_width: u32,
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
        let args = Args::parse();
        let file_config: FileConfig = toml::from_str(
            &std::fs::read_to_string(&args.config)
                .context(format!("Unable to read config file '{:?}'", &args.config))?,
        )?;
        Self::construct(args, file_config)
    }

    fn construct(mut args: Args, file_config: FileConfig) -> Result<ConfigWithEntries> {
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
                command: args.command.take(),
                hide_banner: args.no_banner || file_config.no_banner,
                verbose: args.verbose || file_config.verbose,
                sort: args.sort || file_config.sort,
                preview_width: file_config.preview_width,
                dry_run: args.dry_run,
                default_dir: file_config.default_dir.try_into()?,
            },
            entries,
        ));
    }

    pub fn example_config() -> Result<String> {
        toml::to_string(&FileConfig::default()).context("Unable to serialize example config")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    mod construct {
        use std::{path::PathBuf, str::FromStr, sync::Arc};
        use test_case::test_case;

        use crate::config::{
            file_config::{FileEntry, FileEntryKind},
            Command, Entry, EntryDir, EntryPlain, PreviewCommands,
        };

        use super::super::{args::Args, file_config::FileConfig, Config};

        fn setup() -> (Args, FileConfig) {
            return (Args::default(), FileConfig::default());
        }

        #[test]
        fn entries() {
            let (args, mut file) = setup();
            file.preview_cmd = None;
            file.entries = vec![
                FileEntry {
                    kind: FileEntryKind::Plain,
                    name: "plain name".to_owned(),
                    workdir: "/home/test/".to_owned(),
                    excludes: None,
                    preview_cmd: None,
                },
                FileEntry {
                    kind: FileEntryKind::Dir,
                    name: "plain name".to_owned(),
                    workdir: "/home/test/".to_owned(),
                    excludes: Some(vec!["dir1".to_owned()]),
                    preview_cmd: None,
                },
            ];

            let result = Config::construct(args, file).unwrap();

            assert_eq!(
                result.1[0],
                Entry::Plain(EntryPlain {
                    name: "plain name".to_owned(),
                    workdir: "/home/test/".to_owned().try_into().unwrap(),
                    preview_cmd: None
                })
            );
            assert_eq!(
                result.1[1],
                Entry::Dir(EntryDir {
                    name: "plain name".to_owned(),
                    workdir: "/home/test/".to_owned().try_into().unwrap(),
                    preview_cmd: None,
                    excludes: Some(vec!["dir1".to_owned()]),
                })
            );
        }

        #[test]
        fn default_dir() {
            let (args, mut file) = setup();
            file.default_dir = "some def dir".to_owned();

            let result = Config::construct(args, file).unwrap();

            assert_eq!(result.0.default_dir, "some def dir".to_owned().try_into().unwrap());
        }

        #[test]
        fn preview_width() {
            let (args, mut file) = setup();
            file.preview_width = 99;

            let result = Config::construct(args, file).unwrap();

            assert_eq!(result.0.preview_width, 99);
        }

        #[test]
        fn dry_run() {
            let (mut args, file) = setup();
            args.dry_run = true;

            let result = Config::construct(args, file).unwrap();

            assert!(result.0.dry_run);
        }

        #[test]
        fn correct_config_path() {
            let pathbuf = PathBuf::from_str("/home/test/some/path/file.toml").unwrap();
            let (mut args, file) = setup();
            args.config = pathbuf.clone();

            let result = Config::construct(args, file).unwrap();

            assert_eq!(result.0.config_path, pathbuf);
        }

        #[test]
        fn correct_command() {
            let (mut args, file) = setup();
            args.command = Some(Command::Switch {
                name: "test".to_owned(),
                grouped: true,
            });

            let result = Config::construct(args, file).unwrap();

            assert_eq!(
                result.0.command,
                Some(Command::Switch {
                    name: "test".to_owned(),
                    grouped: true,
                })
            );
        }

        #[test_case(true, false, true ; "disabled in args, enabled in file")]
        #[test_case(false, true, true ; "enabled in args, disabled in file")]
        #[test_case(false, false, false ; "enabled in args, enabled in file")]
        #[test_case(true, true, true ; "disabled in args, disabled in file")]
        fn no_banner(arg_val: bool, file_val: bool, expected: bool) {
            let (mut args, mut file) = setup();
            args.no_banner = arg_val;
            file.no_banner = file_val;

            let result = Config::construct(args, file).unwrap();
            assert_eq!(result.0.hide_banner, expected);
        }

        #[test_case(true, false, true ; "enabled in args, disabled in file")]
        #[test_case(false, true, true ; "disabled in args, enabled in file")]
        #[test_case(false, false, false ; "disabled in args, disabled in file")]
        #[test_case(true, true, true ; "enabled in args, enabled in file")]
        fn verbose(arg_val: bool, file_val: bool, expected: bool) {
            let (mut args, mut file) = setup();
            args.verbose = arg_val;
            file.verbose = file_val;

            let result = Config::construct(args, file).unwrap();
            assert_eq!(result.0.verbose, expected);
        }

        #[test_case(true, false, true ; "enabled in args, disabled in file")]
        #[test_case(false, true, true ; "disabled in args, enabled in file")]
        #[test_case(false, false, false ; "disabled in args, disabled in file")]
        #[test_case(true, true, true ; "enabled in args, enabled in file")]
        fn sort(arg_val: bool, file_val: bool, expected: bool) {
            let (mut args, mut file) = setup();
            args.sort = arg_val;
            file.sort = file_val;

            let result = Config::construct(args, file).unwrap();
            assert_eq!(result.0.sort, expected);
        }

        #[test]
        fn preview_commands_only_args() {
            let (mut args, file) = setup();
            args.preview = Some("preview".to_owned());
            args.preview_no_session = Some("no sess".to_owned());

            let result = Config::construct(args, file).unwrap();

            assert_eq!(
                result.0.preview_commands,
                Some(PreviewCommands {
                    running: Some(Arc::from("preview")),
                    not_running: Some(Arc::from("no sess"))
                })
            )
        }

        #[test]
        fn preview_commands_only_file() {
            let (args, mut file) = setup();
            file.preview_cmd = Some(crate::config::FilePreviewCommands {
                running: Arc::from("preview"),
                not_running: Some(Arc::from("no sess")),
            });

            let result = Config::construct(args, file).unwrap();

            assert_eq!(
                result.0.preview_commands,
                Some(PreviewCommands {
                    running: Some(Arc::from("preview")),
                    not_running: Some(Arc::from("no sess"))
                })
            )
        }

        #[test]
        fn preview_commands_only_args_precedence() {
            let (mut args, mut file) = setup();
            args.preview = Some("preview".to_owned());
            args.preview_no_session = Some("no sess".to_owned());

            file.preview_cmd = Some(crate::config::FilePreviewCommands {
                running: Arc::from("preview file"),
                not_running: Some(Arc::from("no sess file")),
            });

            let result = Config::construct(args, file).unwrap();

            assert_eq!(
                result.0.preview_commands,
                Some(PreviewCommands {
                    running: Some(Arc::from("preview")),
                    not_running: Some(Arc::from("no sess"))
                })
            )
        }
    }
}
