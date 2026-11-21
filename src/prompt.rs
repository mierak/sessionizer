use anyhow::{anyhow, Context, Result};
use skim::{prelude::SkimOptionsBuilder, Skim, SkimOptions};

use crate::{config::Config, prompt_item::PromptItem};

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
    let mut skim_opts = SkimOptionsBuilder::default();

    let preview = format!("right:{}%", config.preview_width);

    skim_opts.preview("");
    skim_opts.preview_window(preview.as_str());
    skim_opts.reverse(true);
    // skim_opts.bind(vec!["ctrl-x:execute(tmux kill-session -t {})", "ctrl-x:refresh-cmd"]);

    let header = gen_header(&config.hide_banner)?;
    skim_opts.header(header);

    return prompt_for_session(entries, skim_opts.build().context("Unable to build skim opts")?);
}

fn gen_header(hide_banner: &bool) -> Result<String> {
    let mut header = if *hide_banner {
        String::new()
    } else {
        String::from(HEADER)
    };
    header.push_str(&format!(
        "{:^3} {:^40} {:^60} {}",
        "*", "Name", "Working Directory", "Window Count",
    ));

    return Ok(header);
}

fn prompt_for_session(entries: Vec<PromptItem>, opts: SkimOptions) -> Result<Option<PromptItem>> {
    // skim reports errors as eyre::Report, which does not convert into anyhow::Error directly.
    let output = Skim::run_items(opts, entries).map_err(|err| anyhow!("Unable to run skim: {err}"))?;
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
