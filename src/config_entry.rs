use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;

use anyhow::Context;
use anyhow::Result;
use skim::ItemPreview;
use skim::SkimItem;

use crate::config::Config;
use crate::config::Entry;
use crate::config::Workdir;
use crate::tmux::SessionStats;
use crate::utils::is_dir;

#[derive(Debug, Clone, PartialEq)]
pub struct PromptItem {
    pub name: String,
    pub workdir: Workdir,
    pub attached: Option<bool>,
    pub window_count: Option<u32>,
}

impl SkimItem for PromptItem {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!(
            "{:<3} {:<40} {:<60} {}",
            self.attached.map_or("", |v| if v { "(*)" } else { "" }),
            self.name,
            self.workdir.as_ref(),
            self.window_count.map_or("".to_owned(), |v| format!("{v} window(s)"))
        ))
    }

    fn output(&self) -> Cow<str> {
        Cow::Owned(format!("{} {}", self.name, self.workdir.as_ref()))
    }

    fn preview(&self, context: skim::PreviewContext) -> skim::ItemPreview {
        if !context.cmd_query.is_empty() {
            ItemPreview::Command(
                context
                    .cmd_query
                    .replace("{{workdir}}", self.workdir.as_ref())
                    .replace("{{name}}", &self.name),
            )
        } else {
            ItemPreview::Text("".to_owned())
        }
    }
}

impl PromptItem {
    pub fn new(name: String, workdir: Workdir) -> Self {
        return PromptItem {
            name,
            workdir,
            window_count: None,
            attached: None,
        };
    }

    pub fn from_entry(
        value: Entry,
        active_sessions: &mut HashMap<String, SessionStats>,
        acc: &mut Vec<PromptItem>,
    ) -> Result<()> {
        match value {
            Entry::Dir {
                name: dir_name,
                workdir,
            } => fs::read_dir(workdir.as_ref())
                .context(format!("Unable to read dir '{}'.", workdir.as_ref()))?
                .filter(is_dir)
                .try_for_each(|entry| -> Result<()> {
                    let entry = entry.context("Unexpected error when reading dir.")?;

                    let dir_path = entry
                        .path()
                        .to_str()
                        .context("Unable to convert path {:?} to str.")?
                        .to_owned();

                    let name = dir_path
                        .split('/')
                        .last()
                        .context("Unable to convert path {:?} to str.")?
                        .to_owned();

                    let mut prompt_item = PromptItem::new(name, dir_path.try_into()?);

                    prompt_item.name = format!("{} - {}", dir_name, prompt_item.name);
                    prompt_item.populate_session_data(active_sessions);
                    active_sessions.remove(&prompt_item.name);
                    acc.push(prompt_item);
                    Ok(())
                }),
            Entry::Plain { name, workdir } => {
                let mut prompt_item = PromptItem::new(name, workdir);
                prompt_item.populate_session_data(active_sessions);
                active_sessions.remove(&prompt_item.name);
                acc.push(prompt_item);
                Ok(())
            }
        }
    }

    pub fn populate_session_data(&mut self, active_sessions: &HashMap<String, SessionStats>) {
        if let Some(s) = active_sessions.get(&self.name) {
            self.attached = Some(s.attached);
            self.window_count = Some(s.window_count);
        }
    }

    pub fn from_active_session(config: &Config, name: String, stats: &SessionStats) -> Self {
        Self {
            workdir: config.default_dir.to_owned(),
            name,
            attached: Some(stats.attached),
            window_count: Some(stats.window_count),
        }
    }
}
