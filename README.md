# Tmux sessionizer

Manage your tmux session with a fuzzy finder and some additional quality of life features. This tool is a personalized and opinionated spin on ThePrimeagen's [tmux-sessionizer](https://github.com/ThePrimeagen/.dotfiles/blob/master/bin/.local/scripts/tmux-sessionizer). Written to get my feet wet in Rust and to add some functionalities to the script that were missing.

Those include:
* Configurability. I want to specify directories to scrape as well as simple sessions with custom work dirs
* Preview of sessions with custom commands
* Attach to a session as a grouped session

Please note that configuration below may not be always up to date as the projects is still changing!

## Installation
```bash
cargo install --git https://github.com/mierak/sessionizer
```
Then assuming cargo bin is on your path:
```bash
tms config -e > "$XDG_CONFIG_HOME/tmux/sessionizer.toml"
# or if you do not have XDG_CONFIG_HOME set
tms config -e > "$HOME/.config/tmux/sessionizer.toml"
```
This will create an example config as seen below. Customize the entries in your favorite editor.

## Usage
I suggest putting following bind in your tmux config:
```
bind f display-popup -E -w 95% -h 95% "tms"
```
Then, while inside tmux, you can press \<prefix\>f and tmux-sessionizer will pop up.

## Example config
Configuration is done via file located at `$XDG_CONFIG_HOME/tmux/sessionizer.toml` or `$HOME/.config/tmux/sessionizer.toml` by default. This can be overriden with the `--config` parameter. Most other options can be overriden on the CLI as well. See `tms -h` for more.

```toml
no_banner = true
verbose = false
sort = true
preview_width = 30
# Default dir is used when switching to session directly via "tms switch" or when session is not found in the entries.
default_dir = "/"

# Preview commands can use {{name}} and {{workdir}} which will be substituted.
[preview_cmd]
running = "tmux capture-pane -pe -t $(tmux list-panes -F '#{pane_id}' -s -t '{{name}}' -f '#{window_active}')"
not_running = "ls -la"

[[entry]] # This table is used to display which sessions you want to manage with fuzzy finder
kind = "Plain" # Plain entry simply displays as is
name = "My session"
workdir = "/"

# You can also specify preview commands on a per entry basis.
[entry.preview_cmd]
running = "ls -la"
not_running = "ls -la"

[[entry]]
kind = "Dir" # Dir entries show all first level subdirectories as plain entries.
name = "My Projects Dir - {{name}} {{workdir}}" # For dir entries, the name is a template which can also use {{name}} and {{workdir}}.
workdir = "/home/youruser"
excludes = ["somedir"] # You can also define directories to exclude.

```

## CLI help
Running `tms -h` will give you following output. Commands have their own help as well.

```
Usage: tms [OPTIONS] [COMMAND]

Commands:
  list    Default behaviour. List all sessions from config and choose which one to switch to
  switch  Directly switches to session
  config  Prints config with placeholder values
  kill
  help    Print this message or the help of the given subcommand(s)

Options:
  -c, --config <FILE>
          [default: /home/<YOUR_USERNAME>/.config/tmux/sessionizer.toml]
      --no-banner
          Disable the big banner in list mode
  -d, --dry-run
          Dry run, dont switch session.
  -e, --eval-mode
          Create the session if needed but do not switch to it. Print the session name to stdout. Useful for scripting. ie. 'tmux switch-client -t $(tms -e)'
  -v, --verbose
          Enable verbose output.
  -s, --sort
          Sort the entries. Running sessions, most windows first.
      --preview <PREVIEW>
          Command to run when prievewing running session
      --preview-no-session <PREVIEW_NO_SESSION>
          Command to run when prievewing session that is not running
  -h, --help
          Print help
```
