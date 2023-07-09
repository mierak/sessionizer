#![allow(clippy::needless_return)]
#![deny(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
pub mod config;
pub mod config_entry;
pub mod prompt;
pub mod tmux;
pub mod utils;

extern crate skim;
use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::Context;
use anyhow::Result;
use config::Command;
use config::Config;
use config::Entry;
use config_entry::PromptItem;
use tmux::Execute;
use tmux::SessionStats;
use tmux::Tmux;

pub fn run<E: Execute>(prompt_items: Vec<PromptItem>, tmux: &Tmux<E>, config: &Config) -> Result<()> {
    match config.command {
        Some(Command::List) | None => run_selection(prompt_items, tmux, config),
        Some(Command::Config) => {
            println!(
                "{}",
                Config::get_dummy_config_file().context("Unable to serialize dummy config")?
            );
            Ok(())
        }
        Some(Command::Switch { session_name: ref name }) => {
            let item = &PromptItem::new(name.to_owned(), config.default_dir.to_owned());
            handle_selected_item(item, tmux, config)
        }
    }
}

fn run_selection<E: Execute>(prompt_items: Vec<PromptItem>, tmux: &Tmux<E>, config: &Config) -> Result<()> {
    match prompt::show(prompt_items, config) {
        Ok(Some(selected_item)) => handle_selected_item(&selected_item, tmux, config),
        Ok(None) => Ok(()), // do nothing, cancelled
        Err(err) => return Err(err),
    }
}

fn handle_selected_item<E: Execute>(item: &PromptItem, tmux: &Tmux<E>, config: &Config) -> Result<()> {
    let tmux_running = tmux.is_tmux_running()?;
    let inside_tmux = std::env::var("TMUX").is_ok();

    if config.verbose {
        println!("Selected_item: {item:?}");
        println!("inside_tmux: {inside_tmux:?}");
        println!("is_tmux_running()?: {tmux_running:?}");
    };

    if config.dry_run {
        return Ok(());
    }

    if !tmux_running && !inside_tmux {
        tmux.new_session(&item.name, item.workdir.as_ref(), false)?.print();
        return Ok(());
    }
    if !tmux.has_session(&item.name)? {
        tmux.new_session(&item.name, item.workdir.as_ref(), true)?.print();
    }

    if !inside_tmux {
        tmux.attach(&item.name)?.print();
    } else {
        tmux.switch_client(&item.name)?.print();
    }
    return Ok(());
}

pub fn create_prompt_items(
    config: &Config,
    entries: Vec<Entry>,
    active_sessions: &mut HashMap<String, SessionStats>,
) -> Result<Vec<PromptItem>> {
    let mut res = entries.into_iter().try_fold(Vec::new(), |mut acc, e| {
        PromptItem::from_entry(e, active_sessions, &mut acc)?;
        Ok::<Vec<PromptItem>, anyhow::Error>(acc)
    })?;

    if !active_sessions.is_empty() {
        res = active_sessions
            .iter()
            .map(|(k, v)| PromptItem::from_active_session(config.default_dir.to_owned(), k.to_owned(), v))
            .chain(res)
            .collect();
    }

    if config.sort {
        #[rustfmt::skip]
        res.sort_by(|a, b| {
            match (a, b) {
                (PromptItem { stats: Some(SessionStats { attached: true, ..}), .. }, _) => Ordering::Less,
                (_, PromptItem { stats: Some(SessionStats { attached: true, ..}), .. }) => Ordering::Greater,
                (PromptItem { stats: Some(_), .. }, PromptItem { stats: None, .. }) => Ordering::Less,
                (PromptItem { stats: None, .. }, PromptItem { stats: Some(_), .. }) => Ordering::Greater,
                (PromptItem { stats: Some(SessionStats {window_count: c1, .. }), .. }, PromptItem { stats: Some(SessionStats {window_count: c2, .. }), .. }) if (c1 == c2) => Ordering::Equal,
                (PromptItem { stats: Some(SessionStats {window_count: c1, .. }), .. }, PromptItem { stats: Some(SessionStats {window_count: c2, .. }), .. }) if (c1 > c2) => Ordering::Less,
                (PromptItem { stats: Some(_), .. }, PromptItem { stats: Some(_), .. }) => Ordering::Greater,
                (PromptItem { name: name1, .. }, PromptItem { name: name2, .. }) => name1.to_lowercase().cmp(&name2.to_lowercase()),

            }
        });
    }

    return Ok(res);
}
