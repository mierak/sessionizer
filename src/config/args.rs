use std::{env, path::PathBuf};

use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug, Default)]
#[command(name = "Tmux Sessionizer")]
#[command(about = "Manage and switch tmux sessions with a fuzzy finder", long_about = None)]
pub(crate) struct Args {
    #[arg(short, long, value_name = "FILE", default_value = get_default_config_path().into_os_string())]
    pub config: PathBuf,

    #[arg(long, default_value_t = false, help = "Disable the big banner in list mode")]
    pub no_banner: bool,

    #[arg(short, long, default_value_t = false, help = "Dry run, dont switch session.")]
    pub dry_run: bool,

    #[arg(short, long, default_value_t = false, help = "Enable verbose output.")]
    pub verbose: bool,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Create the session if needed but do not switch to it. Print the session name to stdout. Useful for scripting. ie. 'tmux switch-client -t $(tms -e)'"
    )]
    pub eval_mode: bool,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Sort the entries. Running sessions, most windows first."
    )]
    pub sort: bool,

    #[arg(long, help = "Command to run when prievewing running session")]
    pub preview: Option<String>,

    #[arg(long, help = "Command to run when prievewing session that is not running")]
    pub preview_no_session: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Clone, Debug, PartialEq)]
pub enum Command {
    /// Default behaviour. List all sessions from config and choose which one to switch to
    List {
        #[arg(long, short, default_value_t = false)]
        grouped: bool,
    },
    /// Directly switches to session
    Switch {
        #[arg(long, short, default_value_t = false)]
        grouped: bool,
        #[arg(
            name = "NAME",
            help = "Session name to switch directly to. Will create the session if it does not exist"
        )]
        name: String,
    },
    /// Prints config with placeholder values
    Config {
        #[arg(short, long, default_value_t = false)]
        example: bool,
    },
    Kill {
        #[arg(short, long, default_value_t = false, group = "kill")]
        current: bool,
        #[arg(short, long, group = "kill")]
        name: Option<String>,
    },
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
