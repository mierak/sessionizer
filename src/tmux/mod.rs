mod executor;

use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;

pub use self::executor::Execute;
use self::executor::Executor;
use self::executor::Output;

pub struct Tmux<E: Execute> {
    verbose: bool,
    executor: E,
}

impl Tmux<Executor> {
    pub fn new(config: &crate::config::Config) -> Self {
        Self {
            executor: Executor,
            verbose: config.verbose,
        }
    }
}

impl Default for Tmux<Executor> {
    fn default() -> Self {
        Self {
            executor: Executor,
            verbose: false,
        }
    }
}

impl<E: Execute> Tmux<E> {
    pub fn new_with_executor(config: &crate::config::Config, executor: E) -> Self {
        Self {
            executor,
            verbose: config.verbose,
        }
    }

    pub fn new_session(&self, session_name: &str, cwd: &str) -> Result<String> {
        self.create_session(&["-s", session_name, "-c", cwd])
    }

    pub fn new_grouped_session(&self, group_session_id: &str) -> Result<String> {
        self.create_session(&["-t", group_session_id])
    }

    pub fn find_session_id(&self, session_name: &str) -> Result<Option<String>> {
        Ok(self
            .list_sessions(None)?
            .value()
            .remove(session_name)
            .map(|stats| stats.id))
    }

    pub fn attach(&self, session_id: &str) -> Result<()> {
        self.executor
            .replace_process("tmux", &["attach", "-t", session_id], self.verbose)
    }

    pub fn switch_client(&self, session_id: &str) -> Result<()> {
        self.execute_checked(&["switch-client", "-t", session_id])?;
        Ok(())
    }

    pub fn kill_session(&self, session_id: &str) -> Result<()> {
        self.execute_checked(&["kill-session", "-t", session_id])?;
        Ok(())
    }

    pub fn get_active_sessions(&self) -> Result<Sessions> {
        let current_session_id = self.current_session_id()?;
        self.list_sessions(current_session_id.as_deref())
    }

    fn list_sessions(&self, current_session_id: Option<&str>) -> Result<Sessions> {
        let output = self.execute(&["list-sessions", "-F", SESSION_FORMAT])?;
        parse_sessions(&String::from_utf8_lossy(&output.stdout), current_session_id)
    }

    fn current_session_id(&self) -> Result<Option<String>> {
        if !is_inside_tmux() {
            return Ok(None);
        }
        let output = self.execute(&["display-message", "-p", "#{session_id}"])?;
        Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned()))
    }

    fn create_session(&self, args: &[&str]) -> Result<String> {
        let mut new_session_args = vec!["new-session", "-d", "-P", "-F", "#{session_id}"];
        new_session_args.extend_from_slice(args);
        let output = self.execute_checked(&new_session_args)?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn execute_checked(&self, args: &[&str]) -> Result<Output> {
        let output = self.execute(args)?;
        if !output.status.success() {
            bail!(
                "tmux {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(output)
    }

    fn execute(&self, args: &[&str]) -> Result<Output> {
        self.executor.execute("tmux", args, self.verbose)
    }
}

pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok_and(|value| !value.is_empty())
}

const SESSION_FORMAT: &str = "#{session_id} #{session_attached} #{session_windows} #{session_name}";

fn parse_sessions(list_sessions_output: &str, current_session_id: Option<&str>) -> Result<Sessions> {
    list_sessions_output
        .lines()
        .map(|line| -> Result<(String, SessionStats)> {
            let malformed = || anyhow!("Unexpected line in tmux list-sessions output: '{line}'");
            let (id, rest) = line.split_once(' ').ok_or_else(malformed)?;
            let (attached_clients, rest) = rest.split_once(' ').ok_or_else(malformed)?;
            let (window_count, name) = rest.split_once(' ').ok_or_else(malformed)?;
            let stats = SessionStats {
                id: id.to_owned(),
                window_count: window_count.parse().with_context(malformed)?,
                attached: attached_clients != "0",
                current: current_session_id == Some(id),
            };
            Ok((name.to_owned(), stats))
        })
        .collect::<Result<HashMap<_, _>>>()
        .map(Sessions)
}

#[derive(Default)]
pub struct Sessions(HashMap<String, SessionStats>);
impl Sessions {
    pub fn value(self) -> HashMap<String, SessionStats> {
        self.0
    }
    pub fn value_ref(&self) -> &HashMap<String, SessionStats> {
        &self.0
    }
    pub fn value_ref_mut(&mut self) -> &mut HashMap<String, SessionStats> {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionStats {
    pub id: String,
    pub window_count: u8,
    pub attached: bool,
    pub current: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{parse_sessions, SessionStats};

    #[test]
    fn parses_sessions_with_spaces_and_dots_in_names() {
        let output = "$0 1 2 rmpc-docs\n$3 0 1 Repos - foo.nvim\n";

        let sessions = parse_sessions(output, Some("$0")).unwrap().value();

        assert_eq!(
            sessions["rmpc-docs"],
            SessionStats {
                id: "$0".to_owned(),
                window_count: 2,
                attached: true,
                current: true,
            }
        );
        assert_eq!(
            sessions["Repos - foo.nvim"],
            SessionStats {
                id: "$3".to_owned(),
                window_count: 1,
                attached: false,
                current: false,
            }
        );
        assert!(!sessions.contains_key("rmpc"));
    }

    #[test]
    fn rejects_malformed_output() {
        assert!(parse_sessions("rmpc-docs: 1 windows (created today)", None).is_err());
    }
}
