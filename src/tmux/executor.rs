use std::process::Stdio;
use std::process::{Command, ExitStatus};

use anyhow::Result;

pub trait Execute {
    fn execute(&self, cmd: &str, args: &[&str], verbose: bool) -> Result<Output> {
        let mut cmd = Command::new(cmd);
        let cmd = cmd.args(args);
        if verbose {
            println!("Executing cmd: '{cmd:?}'");
        }
        let output = cmd.stdin(Stdio::inherit()).output()?;

        return Ok(Output {
            stdout: output.stdout,
            stderr: output.stderr,
            status: output.status,
        });
    }
}
pub struct Executor;
impl Execute for Executor {}

pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: ExitStatus,
}

impl Output {
    pub fn print(&self) {
        if !self.stdout.is_empty() {
            println!("self.stdout = {}", String::from_utf8_lossy(&self.stdout));
        }
        if !self.stderr.is_empty() {
            println!("self.stderr = {}", String::from_utf8_lossy(&self.stderr));
        }
    }
}
