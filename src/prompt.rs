use std::sync::Arc;

use anyhow::{Context, Result};
use skim::{
    prelude::{unbounded, SkimOptionsBuilder},
    Skim, SkimItemReceiver, SkimItemSender, SkimOptions,
};

use crate::{config::Config, config_entry::PromptItem};

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

    let empty_cmd = "".to_owned();
    let cmd = config.preview_cmd.as_ref().unwrap_or(&empty_cmd);
    let preview = &format!("right:{}%:+20-10", &config.preview_width.unwrap_or(30));
    if !cmd.is_empty() {
        skim_opts.preview(Some(""));
        skim_opts.preview_window(Some(preview));
    }
    skim_opts.cmd_query(Some(cmd));
    skim_opts.height(Some("100%"));
    skim_opts.multi(false);
    skim_opts.reverse(true);

    let header = gen_header(&config.hide_banner)?;
    skim_opts.header(Some(&header));

    return prompt_for_session(rx_item, skim_opts.build().context("Unable to build skim opts")?);
}

#[rustfmt::skip]
fn gen_header(hide_banner: &bool) -> Result<String> {
    let mut header = String::new();
    if !hide_banner {
        header.push_str(HEADER);
    }
    header.push_str(&format!(
            "{:^3} {:^40} {:^60} {}",
            "*",
            "Name",
            "Working Directory",
            "Window Count",
        ));
    return Ok(header);
}

pub fn prompt_for_session(rx_item: SkimItemReceiver, opts: SkimOptions) -> Result<Option<PromptItem>> {
    let selected_items = Skim::run_with(&opts, Some(rx_item))
        .filter(|out| !out.is_abort)
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    let selected_items = selected_items
        .iter()
        .map(|selected_item| -> Result<PromptItem> {
            let item = (**selected_item)
                .as_any()
                .downcast_ref::<PromptItem>()
                .context("Unable to downcast selected item to ConfigEntry")?;

            return Ok(item.to_owned());
        })
        .collect::<Result<Vec<PromptItem>>>();

    return Ok(selected_items?.pop());
}
