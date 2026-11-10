//! `comfy` — standalone multimedia CLI for ComfyUI-Rust.
//!
//! Other programs drive generation through a stable JSON contract instead of
//! talking to comfy-server: stdout carries exactly one result object, stderr
//! carries logs, and process exit codes distinguish success (0), runtime
//! failures (1) and usage errors (2).

pub mod args;
pub mod backend;
pub mod commands;
pub mod output;

use args::{command_help, Subcommand, HELP, VERSION};
use commands::dispatch;
use output::{print_failure, print_success};

/// Execute the CLI from an arbitrary argument iterator (the first program-name
/// argument is expected to be included, mirroring `std::env::args()`).
pub fn run(args: impl Iterator<Item = String>) -> i32 {
    let parsed = match args::parse(args.skip(1)) {
        Ok(parsed) => parsed,
        Err(err) => {
            // Subcommand-level `--help` is reported through a sentinel usage
            // error so it stays a successful, non-JSON interactive response.
            if let Some(command) = args::is_help_sentinel(&err) {
                println!("{}", command_help(command).unwrap_or(HELP));
                return 0;
            }
            eprintln!("error: {err}");
            print_failure(&err.0);
            return 2;
        }
    };

    match parsed.command {
        Subcommand::Help(command) => {
            println!(
                "{}",
                command.as_deref().and_then(command_help).unwrap_or(HELP)
            );
            0
        }
        Subcommand::Version => {
            println!("comfy {VERSION}");
            0
        }
        command => {
            let backend = match backend::build(&parsed.globals) {
                Ok(backend) => backend,
                Err(e) => {
                    tracing::error!("backend construction failed: {e}");
                    print_failure(&e.to_string());
                    return 1;
                }
            };
            match dispatch(&backend, &parsed.globals, command) {
                Ok((name, outputs, meta)) => {
                    print_success(&name, outputs, meta);
                    0
                }
                Err(e) => {
                    tracing::error!("command failed: {e}");
                    print_failure(&e.to_string());
                    1
                }
            }
        }
    }
}
