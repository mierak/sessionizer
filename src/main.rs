#![allow(clippy::needless_return)]
extern crate skim;

use tmux_sessionizer::config::Config;
use tmux_sessionizer::tmux::Tmux;
use tmux_sessionizer::{create_prompt_items, run};

use anyhow::Result;

fn main() -> Result<()> {
    let (config, entries) = Config::read()?.value();

    if config.verbose {
        println!("{config:#?}");
    };

    let tmux = Tmux::new(&config);
    let mut active_sessions = tmux.get_active_sessions()?;
    let prompt_items = create_prompt_items(&config, entries, &mut active_sessions)?;

    run(prompt_items, &tmux, &config)
}
