use std::borrow::Cow;
use std::cmp::Ordering;
use std::fs;

use anyhow::Context;
use anyhow::Result;
use skim::ItemPreview;
use skim::SkimItem;

use crate::config::Config;
use crate::config::Entry;
use crate::config::EntryDir;
use crate::config::EntryPlain;
use crate::config::PreviewCommands;
use crate::config::Workdir;
use crate::tmux::SessionStats;
use crate::tmux::Sessions;
use crate::utils::is_dir;

#[derive(Debug, Clone, PartialEq)]
pub struct PromptItem {
    pub name: String,
    pub workdir: Workdir,
    pub stats: Option<SessionStats>,
    preview_cmd: Option<PreviewCommands>,
}

impl Entry {
    fn into_prompt_items<F: FnMut(PromptItem)>(self, sessions: &Sessions, for_each: F) -> Result<()> {
        match self {
            Entry::Dir(e) => e.into_prompt_items(sessions, for_each),
            Entry::Plain(e) => e.into_prompt_items(sessions, for_each),
        }
    }
}

impl SessionStats {
    fn into_prompt_items<F: FnMut(PromptItem)>(self, name: String, workdir: Workdir, mut for_each: F) -> Result<()> {
        for_each(PromptItem {
            workdir,
            name,
            stats: Some(self),
            preview_cmd: None,
        });
        Ok(())
    }
}

impl EntryDir {
    fn into_prompt_items<F: FnMut(PromptItem)>(self, sessions: &Sessions, mut for_each: F) -> Result<()> {
        fs::read_dir(self.workdir.as_ref())
            .context(format!("Unable to read dir '{}'.", self.workdir.as_ref()))?
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

                let is_hidden = name.starts_with('.');
                let is_excluded = self.excludes.as_ref().map_or(false, |v| v.iter().any(|el| *el == name));
                if (is_hidden && !self.show_hidden) || is_excluded {
                    return Ok(());
                }

                let mut prompt_item = PromptItem::new(name, dir_path.try_into()?);
                prompt_item.preview_cmd = self.preview_cmd.to_owned();
                prompt_item.name = self
                    .name
                    .replace("{{workdir}}", self.workdir.as_ref())
                    .replace("{{name}}", &prompt_item.name);
                prompt_item.populate_session_data(sessions);

                for_each(prompt_item);

                Ok(())
            })?;

        Ok(())
    }
}
impl EntryPlain {
    fn into_prompt_items<F: FnMut(PromptItem)>(self, sessions: &Sessions, mut for_each: F) -> Result<()> {
        let mut prompt_item = PromptItem::new(self.name, self.workdir);
        prompt_item.populate_session_data(sessions);
        prompt_item.preview_cmd = self.preview_cmd;

        for_each(prompt_item);

        Ok(())
    }
}

impl PromptItem {
    pub fn new(name: String, workdir: Workdir) -> Self {
        return PromptItem {
            name,
            workdir,
            preview_cmd: None,
            stats: None,
        };
    }

    fn populate_session_data(&mut self, sessions: &Sessions) {
        if let Some(s) = sessions.value_ref().get(&self.name) {
            self.stats = Some(s.to_owned());
        }
    }
}

impl SkimItem for PromptItem {
    fn text(&self) -> Cow<str> {
        match self.stats {
            Some(ref stats) => Cow::Owned(format!(
                "{:<3} {:<40} {:<60} {}",
                if stats.attached { "(*)" } else { "( )" },
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
        Cow::Borrowed(&self.name)
    }

    fn preview(&self, _context: skim::PreviewContext) -> skim::ItemPreview {
        let session_running = self.stats.is_some();

        return match &self.preview_cmd {
            Some(PreviewCommands {
                running: Some(running), ..
            }) if session_running => ItemPreview::Command(
                running
                    .replace("{{workdir}}", self.workdir.as_ref())
                    .replace("{{name}}", &self.name),
            ),
            Some(PreviewCommands {
                not_running: Some(not_running),
                ..
            }) if !session_running => ItemPreview::Command(
                not_running
                    .replace("{{workdir}}", self.workdir.as_ref())
                    .replace("{{name}}", &self.name),
            ),
            _ => ItemPreview::Text("".to_owned()),
        };
    }
}

pub trait IntoPromptItems {
    fn into_prompt_items(self, config: &Config, active_sessions: Sessions) -> Result<Vec<PromptItem>>;
}

impl IntoPromptItems for Vec<Entry> {
    fn into_prompt_items(self, config: &Config, mut sessions: Sessions) -> Result<Vec<PromptItem>> {
        let mut res = self.into_iter().try_fold(Vec::new(), |mut acc, e| {
            e.into_prompt_items(&sessions, |item| acc.push(item))?;
            Ok::<Vec<PromptItem>, anyhow::Error>(acc)
        })?;

        for ele in res.iter() {
            sessions.value_ref_mut().remove(&ele.name);
        }

        if !sessions.value_ref().is_empty() {
            for (k, v) in sessions.value().into_iter() {
                v.into_prompt_items(k.to_owned(), config.default_dir.to_owned(), |item| res.push(item))?;
            }
        }

        if config.sort {
            #[rustfmt::skip]
            res.sort_by(|a, b| {
                match (a, b) {
                    (PromptItem { stats: Some(SessionStats { attached: true, ..}), .. }, _) => Ordering::Less,
                    (_, PromptItem { stats: Some(SessionStats { attached: true, ..}), .. }) => Ordering::Greater,
                    (PromptItem { stats: Some(_), .. }, PromptItem { stats: None, .. }) => Ordering::Less,
                    (PromptItem { stats: None, .. }, PromptItem { stats: Some(_), .. }) => Ordering::Greater,
                    (PromptItem { stats: Some(SessionStats { window_count: c1, .. }), .. }, PromptItem { stats: Some(SessionStats { window_count: c2, .. }), .. }) if (c1 == c2) => Ordering::Equal,
                    (PromptItem { stats: Some(SessionStats { window_count: c1, .. }), .. }, PromptItem { stats: Some(SessionStats { window_count: c2, .. }), .. }) if (c1 > c2) => Ordering::Less,
                    (PromptItem { stats: Some(_), .. }, PromptItem { stats: Some(_), .. }) => Ordering::Greater,
                    (PromptItem { name: name1, .. }, PromptItem { name: name2, .. }) => name1.to_lowercase().cmp(&name2.to_lowercase()),

                }
            });
        }

        return Ok(res);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use test_case::test_case;

    use crate::config::EntryDir;
    use crate::tmux::Sessions;

    #[test_case(false, &["visible"] ; "hidden dirs skipped")]
    #[test_case(true, &[".hidden", "visible"] ; "hidden dirs shown")]
    fn dir_entry_filters_dirs(show_hidden: bool, expected: &[&str]) {
        // Each case gets its own dir because test cases run in parallel.
        let root = std::env::temp_dir().join(format!("sessionizer-test-{}-{show_hidden}", std::process::id()));
        for dir in ["visible", ".hidden", "excluded"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }

        let entry = EntryDir {
            name: "{{name}}".to_owned(),
            workdir: root.to_str().unwrap().to_owned().try_into().unwrap(),
            excludes: Some(vec!["excluded".to_owned()]),
            show_hidden,
            preview_cmd: None,
        };

        let mut names = Vec::new();
        let result = entry.into_prompt_items(&Sessions::default(), |item| names.push(item.name));
        fs::remove_dir_all(&root).unwrap();

        result.unwrap();
        names.sort(); // read_dir order is not guaranteed
        assert_eq!(names, expected);
    }
}
