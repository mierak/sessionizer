#![allow(clippy::needless_return)]
#![deny(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
pub mod config;
mod matching;
pub mod prompt;
pub mod prompt_item;
pub mod tmux;
pub mod utils;

extern crate skim;

use anyhow::Context;
use anyhow::Result;
use config::Command;
use config::Config;
use prompt_item::PromptItem;
use tmux::is_inside_tmux;
use tmux::Execute;
use tmux::Tmux;

pub fn run<E: Execute>(prompt_items: Vec<PromptItem>, tmux: &Tmux<E>, config: &Config) -> Result<()> {
    match config.command {
        Some(Command::List { grouped }) => {
            if let Some(selected_item) = prompt::show_with_current_pinned(prompt_items, config)? {
                if config.dry_run {
                    return Ok(());
                }
                switch_to_selected_item(&selected_item, tmux, config, grouped)?;
            }
            Ok(())
        }
        None => {
            if let Some(selected_item) = prompt::show_with_current_pinned(prompt_items, config)? {
                if config.dry_run {
                    return Ok(());
                }
                switch_to_selected_item(&selected_item, tmux, config, false)?;
            }
            Ok(())
        }
        Some(Command::Config { example }) => {
            if example {
                println!(
                    "{}",
                    Config::example_config().context("Unable to serialize dummy config")?
                );
                return Ok(());
            }

            let content = match std::fs::read_to_string(&config.config_path) {
                Ok(content) => content,
                Err(_) => format!(
                    "Config file was not found at '{}'",
                    config.config_path.to_string_lossy()
                ),
            };
            println!("{content}");
            Ok(())
        }
        Some(Command::Switch { ref name, grouped }) => {
            let item = &PromptItem::new(name.to_owned(), config.default_dir.to_owned());
            switch_to_selected_item(item, tmux, config, grouped)
        }
        Some(Command::Kill { current, .. }) if current => {
            if config.dry_run {
                return Ok(());
            }
            let current_session = prompt_items
                .iter()
                .find_map(|item| item.stats.as_ref().filter(|stats| stats.current))
                .context("Cannot kill current session because tms is not running inside tmux.")?;
            tmux.kill_session(&current_session.id)?;
            Ok(())
        }
        Some(Command::Kill {
            name: Some(ref name), ..
        }) => {
            if config.dry_run {
                return Ok(());
            }

            kill_session_by_name(tmux, name)
        }
        Some(Command::Kill { .. }) => {
            let running_sessions = prompt_items.into_iter().filter(PromptItem::is_running).collect();
            if let Some(selected_item) = prompt::show(running_sessions, config)? {
                kill_session_by_name(tmux, &selected_item.name)?;
            }
            Ok(())
        }
    }
}

fn kill_session_by_name<E: Execute>(tmux: &Tmux<E>, session_name: &str) -> Result<()> {
    let session_id = tmux
        .find_session_id(session_name)?
        .with_context(|| format!("Session '{session_name}' is not running."))?;
    tmux.kill_session(&session_id)
}

fn switch_to_selected_item<E: Execute>(
    item: &PromptItem,
    tmux: &Tmux<E>,
    config: &Config,
    grouped: bool,
) -> Result<()> {
    let inside_tmux = is_inside_tmux();

    if config.verbose {
        println!("Selected_item: {item:?}");
        println!("inside_tmux: {inside_tmux:?}");
    };

    let session_id = match tmux.find_session_id(&item.name)? {
        Some(session_id) => session_id,
        None => tmux.new_session(&item.name, item.workdir.as_ref())?,
    };

    if config.eval_mode {
        println!("{session_id}");
        return Ok(());
    }

    let target_session_id = if grouped {
        tmux.new_grouped_session(&session_id)?
    } else {
        session_id
    };

    if inside_tmux {
        tmux.switch_client(&target_session_id)?;
        return Ok(());
    }

    return tmux.attach(&target_session_id);
}
