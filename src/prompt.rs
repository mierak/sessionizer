use std::fmt::Display;

use anyhow::{anyhow, Context, Result};
use skim::{prelude::SkimOptionsBuilder, Skim, SkimItem, SkimOptions, SkimOutput};

use crate::{config::Config, matching::running_sessions_first_matcher, prompt_item::PromptItem};

#[rustfmt::skip]
static HEADER: &str = concat!(
    r#" /$$$$$$$$ /$$      /$$ /$$   /$$ /$$   /$$        /$$$$$$  /$$$$$$$$  /$$$$$$   /$$$$$$  /$$$$$$  /$$$$$$  /$$   /$$ /$$$$$$ /$$$$$$$$ /$$$$$$$$ /$$$$$$$ "#, "\n",
    r#"|__  $$__/| $$$    /$$$| $$  | $$| $$  / $$       /$$__  $$| $$_____/ /$$__  $$ /$$__  $$|_  $$_/ /$$__  $$| $$$ | $$|_  $$_/|_____ $$ | $$_____/| $$__  $$"#, "\n",
    r#"   | $$   | $$$$  /$$$$| $$  | $$|  $$/ $$/      | $$  \__/| $$      | $$  \__/| $$  \__/  | $$  | $$  \ $$| $$$$| $$  | $$       /$$/ | $$      | $$  \ $$"#, "\n",
    r#"   | $$   | $$ $$/$$ $$| $$  | $$ \  $$$$/       |  $$$$$$ | $$$$$   |  $$$$$$ |  $$$$$$   | $$  | $$  | $$| $$ $$ $$  | $$      /$$/  | $$$$$   | $$$$$$$/"#, "\n",
    r#"   | $$   | $$  $$$| $$| $$  | $$  >$$  $$        \____  $$| $$__/    \____  $$ \____  $$  | $$  | $$  | $$| $$  $$$$  | $$     /$$/   | $$__/   | $$__  $$"#, "\n",
    r#"   | $$   | $$\  $ | $$| $$  | $$ /$$/\  $$       /$$  \ $$| $$       /$$  \ $$ /$$  \ $$  | $$  | $$  | $$| $$\  $$$  | $$    /$$/    | $$      | $$  \ $$"#, "\n",
    r#"   | $$   | $$ \/  | $$|  $$$$$$/| $$  \ $$      |  $$$$$$/| $$$$$$$$|  $$$$$$/|  $$$$$$/ /$$$$$$|  $$$$$$/| $$ \  $$ /$$$$$$ /$$$$$$$$| $$$$$$$$| $$  | $$"#, "\n",
    r#"   |__/   |__/     |__/ \______/ |__/  |__/       \______/ |________/ \______/  \______/ |______/ \______/ |__/  \__/|______/|________/|________/|__/  |__/"#, "\n"
);

pub fn show(entries: Vec<PromptItem>, config: &Config) -> Result<Option<PromptItem>> {
    run_picker(entries, None, config)
}

pub fn show_with_current_pinned(entries: Vec<PromptItem>, config: &Config) -> Result<Option<PromptItem>> {
    let (current, others): (Vec<_>, Vec<_>) = entries.into_iter().partition(PromptItem::is_current);
    run_picker(others, current.into_iter().next(), config)
}

fn run_picker(entries: Vec<PromptItem>, pinned: Option<PromptItem>, config: &Config) -> Result<Option<PromptItem>> {
    let mut skim_opts = SkimOptionsBuilder::default();

    let preview = format!("right:{}%", config.preview_width);

    skim_opts.preview("");
    skim_opts.preview_window(preview.as_str());
    skim_opts.reverse(true);
    // skim_opts.bind(vec!["ctrl-x:execute(tmux kill-session -t {})", "ctrl-x:refresh-cmd"]);

    let header = gen_header(&config.hide_banner, pinned.as_ref())?;
    skim_opts.header(header);

    return prompt_for_session(entries, skim_opts.build().context("Unable to build skim opts")?);
}

fn gen_header(hide_banner: &bool, pinned: Option<&PromptItem>) -> Result<String> {
    let mut header = if *hide_banner {
        String::new()
    } else {
        String::from(HEADER)
    };
    header.push_str(&format!(
        "{:^3} {:^40} {:^60} {}",
        "*", "Name", "Working Directory", "Window Count",
    ));
    if let Some(pinned) = pinned {
        header.push('\n');
        header.push_str(&pinned.text());
    }

    return Ok(header);
}

fn prompt_for_session(entries: Vec<PromptItem>, opts: SkimOptions) -> Result<Option<PromptItem>> {
    let output = run_skim(entries, opts)?;
    if output.is_abort {
        return Ok(None);
    }

    return output
        .selected_items
        .last()
        .map(|selected_item| {
            selected_item
                .downcast_item::<PromptItem>()
                .cloned()
                .context("Unable to downcast selected item to PromptItem")
        })
        .transpose();
}

fn run_skim(entries: Vec<PromptItem>, opts: SkimOptions) -> Result<SkimOutput> {
    let matcher = running_sessions_first_matcher(&opts);
    let mut skim = Skim::new_items(opts, entries).map_err(skim_error)?;
    skim.app_mut().matcher = matcher;

    tokio::runtime::Runtime::new()?.block_on(async move {
        skim.enter().await.map_err(skim_error)?;
        skim.run().await.map_err(skim_error)?;
        Ok::<_, anyhow::Error>(skim.output())
    })
}

fn skim_error(error: impl Display) -> anyhow::Error {
    anyhow!("Unable to run skim: {error}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use skim::SkimItem;

    use super::gen_header;
    use crate::{prompt_item::PromptItem, tmux::SessionStats};

    #[test]
    fn pinned_item_is_appended_below_column_titles() {
        let mut current = PromptItem::new("rmpc-docs".to_owned(), "/".to_owned().try_into().unwrap());
        current.stats = Some(SessionStats {
            id: "$0".to_owned(),
            window_count: 1,
            attached: true,
            current: true,
        });

        let without_pinned = gen_header(&true, None).unwrap();
        let with_pinned = gen_header(&true, Some(&current)).unwrap();

        assert_eq!(without_pinned.lines().count(), 1);
        assert_eq!(with_pinned.lines().count(), 2);
        assert_eq!(with_pinned.lines().last(), Some(current.text().as_ref()));
    }
}
