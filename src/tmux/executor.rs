use std::os::unix::process::CommandExt;
use std::process::Stdio;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result};

pub trait Execute {
    fn replace_process(&self, cmd: &str, args: &[&str], verbose: bool) -> Result<()> {
        let mut cmd = Command::new(cmd);
        cmd.args(args);
        if verbose {
            println!("Replacing process with cmd: '{cmd:?}'");
        }
        let error = cmd.exec();
        Err(error).context(format!("Unable to run '{cmd:?}'"))
    }

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
