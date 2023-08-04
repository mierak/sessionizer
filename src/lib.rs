#![allow(clippy::needless_return)]
#![deny(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
pub mod config;
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
use tmux::Execute;
use tmux::Tmux;

pub fn run<E: Execute>(prompt_items: Vec<PromptItem>, tmux: &Tmux<E>, config: &Config) -> Result<()> {
    match config.command {
        Some(Command::List { grouped }) => {
            if let Some(selected_item) = prompt::show(prompt_items, config)? {
                if config.dry_run {
                    return Ok(());
                }
                switch_to_selected_item(&selected_item, tmux, config, grouped)?;
            }
            Ok(())
        }
        None => {
            if let Some(selected_item) = prompt::show(prompt_items, config)? {
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
                .find(|i| i.stats.as_ref().map_or(false, |s| s.attached))
                .context("Cannot kill current session because no session is attached.")?;
            tmux.kill_session(&current_session.name)?;
            Ok(())
        }
        Some(Command::Kill {
            name: Some(ref name), ..
        }) => {
            if config.dry_run {
                return Ok(());
            }

            tmux.kill_session(name)?;
            Ok(())
        }
        Some(Command::Kill { .. }) => {
            if let Some(selected_item) = prompt::show(prompt_items, config)? {
                tmux.kill_session(&selected_item.name)?;
            }
            Ok(())
        }
    }
}

fn switch_to_selected_item<E: Execute>(
    item: &PromptItem,
    tmux: &Tmux<E>,
    config: &Config,
    grouped: bool,
) -> Result<()> {
    let tmux_running = tmux.is_tmux_running()?;
    let inside_tmux = std::env::var("TMUX").is_ok();

    if config.verbose {
        println!("Selected_item: {item:?}");
        println!("inside_tmux: {inside_tmux:?}");
        println!("is_tmux_running()?: {tmux_running:?}");
    };

    if !tmux_running && !inside_tmux {
        tmux.new_session(&item.name, item.workdir.as_ref(), false)?.print();
        return Ok(());
    }
    if !tmux.has_session(&item.name)? {
        tmux.new_session(&item.name, item.workdir.as_ref(), true)?.print();
    }

    if grouped {
        tmux.new_grouped_session(&item.name)?.print();
    }

    if !inside_tmux {
        tmux.attach(&item.name)?.print();
    } else {
        tmux.switch_client(&item.name)?.print();
    }

    return Ok(());
}
