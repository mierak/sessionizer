mod args;
mod file_config;

use crate::config::{
    args::Args,
    file_config::{FileConfig, FileEntryKind},
};
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use file_config::FileEntry;

pub use args::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct Workdir(String);

#[derive(Debug)]
pub enum Entry {
    Dir { name: String, workdir: Workdir },
    Plain { name: String, workdir: Workdir },
}

impl TryFrom<String> for Workdir {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(crate::utils::envsubst(&value)?))
    }
}

impl AsRef<String> for Workdir {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug)]
pub struct Config {
    pub command: Option<Command>,
    pub hide_banner: bool,
    pub verbose: bool,
    pub sort: bool,
    pub preview_cmd: Option<String>,
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

impl Config {
    pub fn read() -> Result<ConfigWithEntries> {
        let args = Args::parse();
        let file_config: FileConfig = toml::from_str(
            &std::fs::read_to_string(&args.config)
                .context(format!("Unable to read config file '{:?}'", &args.config))?,
        )?;

        return Ok(ConfigWithEntries(
            Config {
                command: args.command,
                hide_banner: args.no_banner || !file_config.banner,
                verbose: args.verbose || file_config.verbose,
                sort: args.sort || file_config.sort,
                preview_cmd: args.preview_cmd.or(file_config.preview_cmd),
                preview_width: file_config.preview_width,
                dry_run: args.dry_run,
                default_dir: file_config.default_dir.try_into()?,
            },
            file_config
                .entries
                .into_iter()
                .map(|e| match e.kind {
                    FileEntryKind::Dir => Ok(Entry::Dir {
                        name: e.name,
                        workdir: e.workdir.try_into()?,
                    }),
                    FileEntryKind::Plain => Ok(Entry::Plain {
                        name: e.name,
                        workdir: e.workdir.try_into()?,
                    }),
                })
                .collect::<Result<Vec<Entry>>>()?,
        ));
    }

    pub fn get_dummy_config_file() -> Result<String> {
        let home = std::env::var("HOME").context("HOME env variable not set")?;
        toml::to_string(&FileConfig {
            default_dir: "/".to_owned(),
            banner: true,
            verbose: false,
            sort: true,
            preview_cmd: Some("ls {{workdir}}".to_owned()),
            preview_width: Some(30),
            entries: vec![
                FileEntry {
                    name: "My session".to_owned(),
                    workdir: "/".to_owned(),
                    kind: FileEntryKind::Plain,
                },
                FileEntry {
                    name: "My Projects Dir".to_owned(),
                    workdir: home,
                    kind: FileEntryKind::Dir,
                },
            ],
        })
        .context("Unable to serialize default config")
    }
}
