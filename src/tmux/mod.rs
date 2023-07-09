mod executor;

use std::collections::HashMap;

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

impl<E: Execute> Tmux<E> {
    pub fn new_with_executor(config: &crate::config::Config, executor: E) -> Self {
        Self {
            executor,
            verbose: config.verbose,
        }
    }

    pub fn new_session(&self, session_name: &str, cwd: &str, detached: bool) -> Result<Output> {
        if detached {
            self.execute(&["new-session", "-ds", session_name, "-c", cwd])
        } else {
            self.execute(&["new-session", "-s", session_name, "-c", cwd])
        }
    }

    pub fn is_tmux_running(&self) -> Result<bool> {
        Ok(self.executor.execute("pgrep", &["tmux"], false)?.status.success())
    }

    pub fn has_session(&self, session_name: &str) -> Result<bool> {
        Ok(self.execute(&["has-session", "-t", session_name])?.status.success())
    }

    pub fn attach(&self, session_name: &str) -> Result<Output> {
        self.execute(&["attach", "-t", session_name])
    }

    pub fn switch_client(&self, session_name: &str) -> Result<Output> {
        self.execute(&["switch-client", "-t", session_name])
    }

    pub fn list_sessions(&self) -> Result<Output> {
        self.execute(&["list-sessions"])
    }

    pub fn get_active_sessions(&self) -> Result<Sessions> {
        String::from_utf8_lossy(&self.list_sessions()?.stdout)
            .lines()
            .try_fold(Sessions(HashMap::new()), |mut acc, input| -> Result<Sessions> {
                let (name, rest) = input.split_once(": ").context("")?;
                let (window_count, rest) = rest.split_once(' ').context("")?;
                let active = rest.contains("attached");
                acc.0.insert(
                    name.to_owned(),
                    SessionStats {
                        window_count: window_count.parse()?,
                        attached: active,
                    },
                );
                Ok(acc)
            })
            .context("")
    }

    fn execute(&self, args: &[&str]) -> Result<Output> {
        self.executor.execute("tmux", args, self.verbose)
    }
}

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
    pub window_count: u8,
    pub attached: bool,
}
