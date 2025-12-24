//! See <https://github.com/matklad/cargo-xtask/>.
//!
//! This binary defines various auxiliary build commands, which are not
//! expressible with just `cargo`. Notably, it provides tests via `cargo test -p xtask`
//! for code generation and `cargo xtask install` for installation of
//! rust-analyzer server and client.
//!
//! This binary is integrated into the `cargo` command line by using an alias in
//! `.cargo/config`.
mod build_py;
mod flags;
// mod vendor;

use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;
use xshell::{cmd, Shell};

// mod cache;
mod msvcrt;

fn main() -> Result<()> {
    let mut sh = Shell::new()?;
    sh.change_dir(project_root());

    let flags = flags::Xtask::from_env()?;
    match flags.subcommand {
        flags::XtaskCmd::CargoBuild(cmd) => cmd.run(&sh),
        flags::XtaskCmd::CargoTest(cmd) => cmd.run(&sh),
        flags::XtaskCmd::Verilogae(cmd) => cmd.run(&mut sh),
        flags::XtaskCmd::GenMsvcrt(cmd) => cmd.run(&sh),
    }
}

impl flags::CargoBuild {
    fn run(&self, sh: &Shell) -> Result<()> {
        run_cargo_command(sh, "build", self.release, &self.args)
    }
}

impl flags::CargoTest {
    fn run(&self, sh: &Shell) -> Result<()> {
        run_cargo_command(sh, "test", self.release, &self.args)
    }
}

fn run_cargo_command(
    sh: &Shell,
    subcommand: &str,
    release: bool,
    extra_args: &[String],
) -> Result<()> {
    // Auto-detect LLVM on macOS via Homebrew
    let llvm_prefix = detect_llvm_prefix();

    let mut envs: Vec<(&str, String)> = Vec::new();

    if let Some(prefix) = &llvm_prefix {
        eprintln!("Auto-detected LLVM 18 at: {}", prefix);
        envs.push(("LLVM_SYS_181_PREFIX", prefix.clone()));
    }

    let mut args = Vec::new();
    if release {
        args.push("--release".to_string());
    }
    args.extend(extra_args.iter().cloned());

    let mut command = cmd!(sh, "cargo {subcommand} {args...}");
    for (key, value) in envs {
        command = command.env(key, value);
    }

    command.run()?;
    Ok(())
}

/// Detect LLVM 18 prefix on macOS via Homebrew.
/// Returns None on non-macOS or if HOMEBREW_PREFIX is not set.
fn detect_llvm_prefix() -> Option<String> {
    // Only attempt detection on macOS
    if !cfg!(target_os = "macos") {
        return None;
    }

    // Check if user already set LLVM_SYS_181_PREFIX
    if env::var("LLVM_SYS_181_PREFIX").is_ok() {
        eprintln!("Using existing LLVM_SYS_181_PREFIX from environment");
        return None;
    }

    // Try to get HOMEBREW_PREFIX
    let homebrew_prefix = env::var("HOMEBREW_PREFIX").ok()?;

    let llvm_path = format!("{}/opt/llvm@18", homebrew_prefix);
    let llvm_bin = format!("{}/bin/llvm-config", llvm_path);

    // Verify LLVM is actually installed there
    if Path::new(&llvm_bin).exists() {
        Some(llvm_path)
    } else {
        eprintln!("Note: HOMEBREW_PREFIX is set but llvm@18 not found at {}", llvm_path);
        eprintln!("Install with: brew install llvm@18");
        None
    }
}

fn project_root() -> PathBuf {
    Path::new(
        &env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned()),
    )
    .ancestors()
    .nth(1)
    .unwrap()
    .to_path_buf()
}
