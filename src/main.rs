#![allow(clippy::needless_return)]
extern crate skim;

use tmux_sessionizer::config::Config;
use tmux_sessionizer::prompt_item::IntoPromptItems;
use tmux_sessionizer::run;
use tmux_sessionizer::tmux::Tmux;

use anyhow::Result;

fn main() -> Result<()> {
    let (config, entries) = Config::read()?.value();

    if config.verbose {
        println!("{config:#?}");
    };

    let tmux = Tmux::new(&config);
    let active_sessions = tmux.get_active_sessions()?;
    let prompt_items = entries.into_prompt_items(&config, active_sessions)?;

    run(prompt_items, &tmux, &config)
}
