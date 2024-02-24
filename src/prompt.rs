use std::sync::Arc;

use anyhow::{Context, Result};
use skim::{
    prelude::{unbounded, SkimOptionsBuilder},
    Skim, SkimItemReceiver, SkimItemSender, SkimOptions,
};

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
    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for ele in entries {
        tx_item.send(Arc::new(ele))?;
    }
    drop(tx_item);

    let mut skim_opts = SkimOptionsBuilder::default();

    let preview = &format!("right:{}%", &config.preview_width);

    skim_opts.cmd_query(Some(""));
    skim_opts.preview(Some(""));
    skim_opts.preview_window(Some(preview));
    skim_opts.height(Some("100%"));
    skim_opts.multi(false);
    skim_opts.reverse(true);
    // skim_opts.bind(vec!["ctrl-x:execute(tmux kill-session -t {})", "ctrl-x:refresh-cmd"]);

    let header = gen_header(&config.hide_banner)?;
    skim_opts.header(Some(&header));

    return prompt_for_session(rx_item, skim_opts.build().context("Unable to build skim opts")?);
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

fn prompt_for_session(rx_item: SkimItemReceiver, opts: SkimOptions) -> Result<Option<PromptItem>> {
    let selected_items = Skim::run_with(&opts, Some(rx_item))
        .filter(|out| !out.is_abort)
        .map(|out| out.selected_items)
        .unwrap_or_default();

    let selected_items = selected_items
        .into_iter()
        .map(|selected_item| -> Result<PromptItem> {
            let item = (*selected_item)
                .as_any()
                .downcast_ref::<PromptItem>()
                .context("Unable to downcast selected item to ConfigEntry")?;

            return Ok(item.to_owned());
        })
        .collect::<Result<Vec<PromptItem>>>();

    return Ok(selected_items?.pop());
}
