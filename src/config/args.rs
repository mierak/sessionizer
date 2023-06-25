use std::{env, path::PathBuf};

use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "Tmux Sessionizer")]
#[command(about = "Manage and switch tmux sessions with a fuzzy finder", long_about = None)]
pub(crate) struct Args {
    #[arg(short, long, value_name = "FILE", default_value = get_default_config_path().into_os_string())]
    pub config: PathBuf,

    #[arg(long, default_value_t = false, help = "Disable the big banner in list mode")]
    pub no_banner: bool,

    #[arg(short, long, default_value_t = false, help = "Dry run, dont switch session.")]
    pub dry_run: bool,

    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    #[arg(short, long, default_value_t = false)]
    pub sort: bool,

    #[arg(long, help = "Command to run when prievewing session")]
    pub preview_cmd: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Default behaviour. List all sessions from config and choose which one to switch to
    List,
    /// Directly switches to session
    Switch {
        #[arg(
            name = "NAME",
            help = "Session name to switch directly to. Will create the session if it does not exist"
        )]
        session_name: String,
    },
    /// Prints config with placeholder values
    Config,
}

fn get_default_config_path() -> PathBuf {
    let mut path = PathBuf::new();
    if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
        path.push(dir);
    } else if let Ok(home) = env::var("HOME") {
        path.push(home);
        path.push(".config");
    } else {
        return path;
    }
    path.push("tmux");
    path.push("sessionizer.toml");
    return path;
}
