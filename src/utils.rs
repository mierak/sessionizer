use std::env;

use anyhow::{Context, Result};

pub fn envsubst(input: &str) -> Result<String> {
    Ok(input
        .split('/')
        .map(|word| {
            if let Some(s) = word.strip_prefix('$') {
                return env::var(s).context(format!("Failed to substitute env variable '{s}'"));
            } else if word == "~" {
                return env::var("HOME").context("Failed to substitute HOME for tilde");
            };
            return Ok(word.to_string());
        })
        .collect::<Result<Vec<String>>>()?
        .join("/"))
}

pub fn is_dir(entry: &Result<std::fs::DirEntry, std::io::Error>) -> bool {
    if let Ok(entry) = entry {
        if let Ok(metadata) = entry.metadata() {
            return metadata.is_dir();
        }
    }
    return false;
}
