use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;

use anyhow::Context;
use anyhow::Result;
use skim::ItemPreview;
use skim::SkimItem;

use crate::config::Entry;
use crate::config::Workdir;
use crate::tmux::SessionStats;
use crate::utils::is_dir;

#[derive(Debug, Clone, PartialEq)]
pub struct PromptItem {
    pub name: String,
    pub workdir: Workdir,
    pub stats: Option<SessionStats>,
}

impl PromptItem {
    pub fn new(name: String, workdir: Workdir) -> Self {
        return PromptItem {
            name,
            workdir,
            stats: None,
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
            self.stats = Some(s.to_owned());
        }
    }

    pub fn from_active_session(workdir: Workdir, name: String, stats: &SessionStats) -> Self {
        Self {
            workdir,
            name,
            stats: Some(stats.to_owned()),
        }
    }
}

impl SkimItem for PromptItem {
    fn text(&self) -> Cow<str> {
        match self.stats {
            Some(ref stats) => Cow::Owned(format!(
                "{:<3} {:<40} {:<60} {}",
                if stats.attached { "(*)" } else { "" },
                self.name,
                self.workdir.as_ref(),
                format_args!("{} window(s)", stats.window_count)
            )),
            None => Cow::Owned(format!(
                "{:<3} {:<40} {:<60} {}",
                "",
                self.name,
                self.workdir.as_ref(),
                ""
            )),
        }
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
