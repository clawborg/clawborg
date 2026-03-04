//! Terminal UI helpers: banner, startup animation, spinner, styled output.
//!
//! All functions check `is_tty()` before emitting ANSI codes or control
//! sequences so piped output stays plain text.

use colored::Colorize;
use std::io::{IsTerminal, Write};
use std::time::Duration;

// ─── TTY detection ────────────────────────────────────────────────────────────

fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

// ─── ASCII banner ─────────────────────────────────────────────────────────────

const BANNER: &str = r"
  ██████╗██╗      █████╗ ██╗    ██╗██████╗  ██████╗ ██████╗  ██████╗
 ██╔════╝██║     ██╔══██╗██║    ██║██╔══██╗██╔═══██╗██╔══██╗██╔════╝
 ██║     ██║     ███████║██║ █╗ ██║██████╔╝██║   ██║██████╔╝██║  ███╗
 ██║     ██║     ██╔══██║██║███╗██║██╔══██╗██║   ██║██╔══██╗██║   ██║
 ╚██████╗███████╗██║  ██║╚███╔███╔╝██████╔╝╚██████╔╝██║  ██║╚██████╔╝
  ╚═════╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚═════╝  ╚═════╝ ╚═╝  ╚═╝ ╚═════╝";

pub fn print_banner(version: &str) {
    if is_tty() {
        println!("{}", BANNER.bright_green());
        println!(
            "  {} · The fast, single-binary dashboard for OpenClaw AI agent fleets",
            format!("v{}", version).dimmed()
        );
        println!(
            "  {}  ·  {}  ·  AGPL-3.0",
            "https://clawborg.dev".cyan(),
            "github.com/clawborg/clawborg".cyan()
        );
        println!();
    } else {
        println!("ClawBorg v{}", version);
    }
}

// ─── Foreground startup animation ────────────────────────────────────────────
//
// Two usage patterns:
//
// Pattern A — result already known (one-shot, includes 220 ms fake delay):
//   match do_fast_work() {
//       Ok(_) => ui::startup_step_ok("Step label", "detail"),
//       Err(e) => { ui::startup_step_err("Step label", &e.to_string()); return Err(e); }
//   }
//
// Pattern B — wraps real work (begin/finish, no artificial delay):
//   ui::startup_step_begin("Step label");
//   let result = do_real_work();
//   match result {
//       Ok(_) => ui::startup_step_finish_ok("Step label", "detail"),
//       Err(e) => { ui::startup_step_finish_err("Step label", &e.to_string()); return Err(e); }
//   }

/// One-shot: print begin state, sleep 220 ms (so the user sees it), overwrite with ✓.
/// Use when the work is already done and you just want the animation effect.
pub fn startup_step_ok(label: &str, detail: &str) {
    if is_tty() {
        print!("  {} {}...", "▸".dimmed(), label);
        let _ = std::io::stdout().flush();
        std::thread::sleep(Duration::from_millis(220));
        println!("\r  {} {:<44}{}", "▸".dimmed(), label, step_ok_suffix(detail));
    } else if detail.is_empty() {
        println!("  ▸ {}... ✓", label);
    } else {
        println!("  ▸ {}... ✓  {}", label, detail);
    }
}

/// One-shot error: print ✗ line. Use when work has already failed.
pub fn startup_step_err(label: &str, err: &str) {
    if is_tty() {
        println!("  {} {:<44}{}", "▸".dimmed(), label, format!("✗  {}", err).red());
    } else {
        println!("  ▸ {}... ✗  {}", label, err);
    }
}

/// Two-phase begin: print `▸ label...` and flush. Follow with `startup_step_finish_*`.
pub fn startup_step_begin(label: &str) {
    if is_tty() {
        print!("  {} {}...", "▸".dimmed(), label);
        let _ = std::io::stdout().flush();
    } else {
        print!("  ▸ {}...", label);
        let _ = std::io::stdout().flush();
    }
}

/// Two-phase finish (success): overwrite the begin line with ✓.
pub fn startup_step_finish_ok(label: &str, detail: &str) {
    if is_tty() {
        println!("\r  {} {:<44}{}", "▸".dimmed(), label, step_ok_suffix(detail));
    } else if detail.is_empty() {
        println!(" ✓");
    } else {
        println!(" ✓  {}", detail);
    }
}

/// Two-phase finish (error): overwrite the begin line with ✗.
#[allow(dead_code)]
pub fn startup_step_finish_err(label: &str, err: &str) {
    if is_tty() {
        println!("\r  {} {:<44}{}", "▸".dimmed(), label, format!("✗  {}", err).red());
    } else {
        println!(" ✗  {}", err);
    }
}

fn step_ok_suffix(detail: &str) -> String {
    if detail.is_empty() {
        "✓".green().to_string()
    } else {
        format!("✓  {}", detail).green().to_string()
    }
}

pub fn startup_ready(port: u16) {
    if is_tty() {
        println!(
            "  {} Server ready at {}",
            "▸".dimmed(),
            format!("http://localhost:{}", port).cyan().bold()
        );
        println!();
    } else {
        println!("  ▸ Server ready at http://localhost:{}", port);
    }
}

// ─── Spinner (daemon start) ───────────────────────────────────────────────────

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    label: String,
    frame: usize,
}

impl Spinner {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            frame: 0,
        }
    }

    /// Advance one frame and sleep 80 ms. Call in a loop while work happens.
    pub fn tick(&mut self) {
        if !is_tty() {
            return;
        }
        let frame = SPINNER_FRAMES[self.frame % SPINNER_FRAMES.len()];
        print!("\r  {} {}...", frame.cyan(), self.label);
        let _ = std::io::stdout().flush();
        self.frame += 1;
        std::thread::sleep(Duration::from_millis(80));
    }

    /// Replace spinner line with a success message.
    pub fn finish_ok(self, msg: &str) {
        if is_tty() {
            // Pad to 60 chars to fully overwrite the spinner line
            println!("\r  {} {:<60}", "✓".green(), msg);
        } else {
            println!("  ✓ {}", msg);
        }
    }

    /// Replace spinner line with an error message.
    pub fn finish_err(self, msg: &str) {
        if is_tty() {
            println!("\r  {} {:<60}", "✗".red(), msg);
        } else {
            println!("  ✗ {}", msg);
        }
    }
}

// ─── Styled stop output ───────────────────────────────────────────────────────

/// Print `  ▸ Stopping ClawBorg (PID: N)...` with no trailing newline.
/// Follow with `print_stopped()` or `print_stop_timeout()` after the wait loop.
pub fn print_stopping(pid: u32) {
    if is_tty() {
        print!(
            "  {} Stopping ClawBorg (PID: {})...",
            "▸".dimmed(),
            pid.to_string().yellow()
        );
        let _ = std::io::stdout().flush();
    } else {
        print!("  ▸ Stopping ClawBorg (PID: {})...", pid);
        let _ = std::io::stdout().flush();
    }
}

/// Append `  ✓ stopped` to the current line (after print_stopping).
pub fn print_stopped() {
    if is_tty() {
        println!("  {}", "✓ stopped".green());
    } else {
        println!("  ✓ stopped");
    }
}

pub fn print_not_running() {
    println!("ClawBorg is not running");
}
