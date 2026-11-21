use std::borrow::Cow;
use std::cmp::Reverse;
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

                let name = entry
                    .file_name()
                    .to_str()
                    .context("Unable to convert dir name to str.")?
                    .to_owned();

                let is_hidden = name.starts_with('.');
                let is_excluded = self.excludes.as_ref().is_some_and(|excludes| excludes.contains(&name));
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

    pub fn is_running(&self) -> bool {
        self.stats.is_some()
    }

    pub fn is_current(&self) -> bool {
        self.stats.as_ref().is_some_and(|stats| stats.current)
    }

    fn populate_session_data(&mut self, sessions: &Sessions) {
        if let Some(s) = sessions.value_ref().get(&self.name) {
            self.stats = Some(s.to_owned());
        }
    }
}

impl SkimItem for PromptItem {
    fn text(&self) -> Cow<'_, str> {
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

    fn output(&self) -> Cow<'_, str> {
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

        let mut sessions_not_in_config: Vec<_> = sessions.value().into_iter().collect();
        sessions_not_in_config.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));
        for (name, stats) in sessions_not_in_config {
            stats.into_prompt_items(name, config.default_dir.to_owned(), |item| res.push(item))?;
        }

        order_items(&mut res, config.sort);

        return Ok(res);
    }
}

fn order_items(items: &mut [PromptItem], sort: bool) {
    if sort {
        items.sort_by_cached_key(|item| {
            let stats = item.stats.as_ref();
            (
                Reverse(stats.is_some_and(|stats| stats.attached)),
                Reverse(stats.map(|stats| stats.window_count)),
                item.name.to_lowercase(),
                item.name.clone(),
            )
        });
    }

    items.sort_by_key(|item| (item.is_current(), !item.is_running()));
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::fs;
    use test_case::test_case;

    use super::{order_items, PromptItem};
    use crate::config::EntryDir;
    use crate::tmux::{SessionStats, Sessions};

    fn not_running(name: &str) -> PromptItem {
        PromptItem::new(name.to_owned(), "/".to_owned().try_into().unwrap())
    }

    fn running(name: &str, window_count: u8, attached: bool, current: bool) -> PromptItem {
        let mut item = not_running(name);
        item.stats = Some(SessionStats {
            id: format!("${name}"),
            window_count,
            attached,
            current,
        });
        item
    }

    fn names(items: &[PromptItem]) -> Vec<&str> {
        items.iter().map(|item| item.name.as_str()).collect()
    }

    #[test]
    fn sort_puts_attached_then_most_windows_then_name_and_current_last() {
        let mut items = vec![
            not_running("b"),
            running("few windows", 1, false, false),
            running("other terminal 2", 1, true, false),
            not_running("A"),
            running("many windows", 5, false, false),
            running("other terminal 1", 1, true, false),
            running("here", 1, true, true),
        ];

        order_items(&mut items, true);

        assert_eq!(
            names(&items),
            [
                "other terminal 1",
                "other terminal 2",
                "many windows",
                "few windows",
                "A",
                "b",
                "here"
            ]
        );
    }

    #[test]
    fn without_sorting_running_sessions_come_first_and_current_last() {
        let mut items = vec![
            running("here", 1, true, true),
            not_running("x"),
            running("y", 3, false, false),
            not_running("w"),
            running("z", 1, false, false),
        ];

        order_items(&mut items, false);

        assert_eq!(names(&items), ["y", "z", "x", "w", "here"]);
    }

    #[test_case(false, &["visible"] ; "hidden dirs skipped")]
    #[test_case(true, &[".hidden", "visible"] ; "hidden dirs shown")]
    fn dir_entry_filters_dirs(show_hidden: bool, expected: &[&str]) {
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
        names.sort();
        assert_eq!(names, expected);
    }
}
