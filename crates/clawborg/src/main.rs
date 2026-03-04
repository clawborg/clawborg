use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod cache;
mod clawborg_config;
mod openclaw;
mod routes;
mod server;
mod types;
mod ui;
mod watcher;
mod ws;

// ─── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "clawborg",
    version,
    about = "Dashboard for OpenClaw AI agent fleets",
    long_about = None
)]
struct Cli {
    /// Port for the dashboard
    #[arg(short, long, default_value_t = 3104)]
    port: u16,

    /// OpenClaw directory path (default: ~/.openclaw)
    #[arg(short, long, env = "OPENCLAW_DIR")]
    dir: Option<PathBuf>,

    /// Disable filesystem watching
    #[arg(long, default_value_t = false)]
    no_watch: bool,

    /// Disable write operations (read-only mode)
    #[arg(long, default_value_t = false)]
    readonly: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run workspace health check (CLI output, no server)
    Health,
    /// Print discovered agents and their workspace paths
    Agents,
    /// Print version info
    Version,
    /// Start ClawBorg as a background daemon
    Start,
    /// Stop the running ClawBorg daemon
    Stop,
    /// View daemon log output
    Log {
        /// Follow log output in real time (like tail -f)
        #[arg(short, long)]
        follow: bool,
    },
}

// ─── Path helpers ─────────────────────────────────────────────────────────────

fn clawborg_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".clawborg")
}

fn pid_file_path() -> PathBuf {
    clawborg_dir().join("clawborg.pid")
}

fn log_file_path() -> PathBuf {
    clawborg_dir().join("logs").join("clawborg.log")
}

fn resolve_openclaw_dir(dir: &Option<PathBuf>) -> PathBuf {
    dir.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".openclaw")
    })
}

// ─── PID file helpers ─────────────────────────────────────────────────────────

fn read_pid() -> Option<u32> {
    std::fs::read_to_string(pid_file_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check if a process is alive by sending signal 0 (no-op probe).
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

// ─── Daemon subcommands (all sync — fork must happen before tokio threads) ────

fn cmd_start(
    port: u16,
    openclaw_dir: PathBuf,
    no_watch: bool,
    readonly: bool,
) -> anyhow::Result<()> {
    let pid_file = pid_file_path();
    let log_file = log_file_path();

    ui::print_banner(env!("CARGO_PKG_VERSION"));

    // Validate openclaw directory before forking so errors are visible
    if !openclaw_dir.exists() {
        eprintln!();
        eprintln!("❌ OpenClaw directory not found: {}", openclaw_dir.display());
        eprintln!();
        eprintln!("  Options:");
        eprintln!("    clawborg start --dir /path/to/.openclaw");
        eprintln!("    export OPENCLAW_DIR=/path/to/.openclaw");
        eprintln!();
        std::process::exit(1);
    }

    // Check if already running
    if let Some(existing_pid) = read_pid() {
        if is_process_alive(existing_pid) {
            eprintln!(
                "ClawBorg is already running (PID: {})",
                existing_pid
            );
            eprintln!("  Dashboard: http://localhost:{}", port);
            eprintln!("  Stop with: clawborg stop");
            std::process::exit(1);
        }
        // Stale PID file — remove and continue
        let _ = std::fs::remove_file(&pid_file);
    }

    // Create log directory before forking so errors surface in the terminal
    if let Some(log_dir) = log_file.parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    std::fs::create_dir_all(pid_file.parent().unwrap_or(&clawborg_dir()))?;

    // Brief spinner while we fork
    let mut spinner = ui::Spinner::new("Starting ClawBorg daemon");
    for _ in 0..4 {
        spinner.tick();
    }

    // Fork — MUST happen before any tokio threads are created
    let child_pid = unsafe { libc::fork() };
    match child_pid {
        -1 => {
            spinner.finish_err("fork() failed");
            Err(anyhow::anyhow!(
                "fork() failed: {}",
                std::io::Error::last_os_error()
            ))
        }
        0 => {
            // ── Child process ──
            // Become session leader (detach from controlling terminal)
            unsafe { libc::setsid() };

            // Redirect stdin/stdout/stderr: stdin → /dev/null, stdout/stderr → log file
            redirect_stdio(&log_file)?;

            // Write our PID to the PID file
            let daemon_pid = std::process::id();
            std::fs::write(&pid_file, format!("{}\n", daemon_pid))?;

            // Initialize tracing (stdout now points to log file)
            init_tracing();

            // Build and run the async runtime
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(run_server(port, openclaw_dir, no_watch, readonly, Some(pid_file), false))?;

            Ok(())
        }
        pid => {
            // ── Parent process ──
            spinner.finish_ok(&format!(
                "ClawBorg started (PID: {}) — dashboard at http://localhost:{}",
                pid, port
            ));
            std::process::exit(0);
        }
    }
}

fn cmd_stop() -> anyhow::Result<()> {
    let pid_file = pid_file_path();

    let pid = match read_pid() {
        Some(p) => p,
        None => {
            ui::print_not_running();
            return Ok(());
        }
    };

    if !is_process_alive(pid) {
        let _ = std::fs::remove_file(&pid_file);
        ui::print_not_running();
        return Ok(());
    }

    ui::print_stopping(pid);

    // Send SIGTERM
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if ret != 0 {
        println!(); // newline after the partial "Stopping..." line
        return Err(anyhow::anyhow!(
            "kill() failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    // Wait up to 5 seconds for graceful shutdown
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !is_process_alive(pid) {
            let _ = std::fs::remove_file(&pid_file);
            ui::print_stopped();
            return Ok(());
        }
    }

    println!(); // newline after the partial "Stopping..." line
    eprintln!("ClawBorg (PID: {}) did not stop within 5 seconds", pid);
    eprintln!("Force-kill with: kill -9 {}", pid);
    std::process::exit(1);
}

fn cmd_log(follow: bool) -> anyhow::Result<()> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let log_file = log_file_path();
    if !log_file.exists() {
        println!("No log file found.");
        println!("Start ClawBorg with `clawborg start` to enable logging.");
        return Ok(());
    }

    // Print last 50 lines
    let content = std::fs::read_to_string(&log_file)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(50);
    for line in &lines[start..] {
        println!("{}", line);
    }

    if !follow {
        return Ok(());
    }

    // Follow mode: seek to current end, then poll for new bytes
    let mut file = std::fs::File::open(&log_file)?;
    file.seek(SeekFrom::End(0))?;

    let stdout = std::io::stdout();
    loop {
        let mut buf = [0u8; 4096];
        match file.read(&mut buf) {
            Ok(0) => std::thread::sleep(std::time::Duration::from_millis(250)),
            Ok(n) => {
                stdout.lock().write_all(&buf[..n])?;
                stdout.lock().flush()?;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

// ─── Stdio redirection for daemon mode ───────────────────────────────────────

fn redirect_stdio(log_path: &std::path::Path) -> anyhow::Result<()> {
    use std::os::unix::io::IntoRawFd;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let log_fd = log_file.into_raw_fd();

    unsafe {
        libc::dup2(log_fd, libc::STDOUT_FILENO);
        libc::dup2(log_fd, libc::STDERR_FILENO);
        libc::close(log_fd);

        let devnull = c"/dev/null".as_ptr();
        let null_fd = libc::open(devnull, libc::O_RDONLY);
        if null_fd >= 0 {
            libc::dup2(null_fd, libc::STDIN_FILENO);
            libc::close(null_fd);
        }
    }

    Ok(())
}

// ─── Shared tracing setup ─────────────────────────────────────────────────────

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("clawborg=info")),
        )
        .init();
}

// ─── Async server runner (shared by foreground and daemon) ────────────────────

async fn run_server(
    port: u16,
    openclaw_dir: PathBuf,
    no_watch: bool,
    readonly: bool,
    pid_file: Option<PathBuf>,
    animate: bool,
) -> anyhow::Result<()> {
    let config = server::ServerConfig {
        port,
        openclaw_dir,
        watch_enabled: !no_watch,
        readonly,
        animate,
    };

    tokio::select! {
        result = server::run(config) => {
            if let Some(ref pf) = pid_file {
                let _ = std::fs::remove_file(pf);
            }
            result
        }
        _ = shutdown_signal() => {
            tracing::info!("Shutting down gracefully");
            if let Some(ref pf) = pid_file {
                let _ = std::fs::remove_file(pf);
            }
            Ok(())
        }
    }
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(_) => {
            tokio::signal::ctrl_c().await.ok();
            return;
        }
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = sigterm.recv() => {},
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle sync-only commands BEFORE the async runtime starts.
    // CRITICAL: fork() is unsafe after tokio threads exist, so `start` is also
    // handled here — fork happens inside cmd_start before the runtime is built.
    match &cli.command {
        Some(Commands::Stop) => return cmd_stop(),
        Some(Commands::Log { follow }) => return cmd_log(*follow),
        Some(Commands::Start) => {
            let dir = resolve_openclaw_dir(&cli.dir);
            return cmd_start(cli.port, dir, cli.no_watch, cli.readonly);
        }
        _ => {}
    }

    // Banner for foreground server mode only (not health/agents/version)
    if cli.command.is_none() {
        ui::print_banner(env!("CARGO_PKG_VERSION"));
    }

    // Initialize tracing for foreground / CLI commands (writes to terminal)
    init_tracing();

    let openclaw_dir = resolve_openclaw_dir(&cli.dir);

    // Validate directory for commands that need it (all except Version)
    if !openclaw_dir.exists() && !matches!(cli.command, Some(Commands::Version)) {
        eprintln!();
        eprintln!("❌ OpenClaw directory not found: {}", openclaw_dir.display());
        eprintln!();
        eprintln!("  ClawBorg needs an OpenClaw installation to read from.");
        eprintln!();
        eprintln!("  Options:");
        eprintln!("    clawborg --dir /path/to/.openclaw        # point to your install");
        eprintln!("    clawborg --dir ./fixtures/mock-openclaw   # use mock data for testing");
        eprintln!();
        eprintln!("  Set OPENCLAW_DIR environment variable to make it permanent:");
        eprintln!("    export OPENCLAW_DIR=/path/to/.openclaw");
        eprintln!();
        std::process::exit(1);
    }

    tracing::info!("📂 OpenClaw directory: {}", openclaw_dir.display());

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async_main(cli, openclaw_dir))
}

async fn async_main(cli: Cli, openclaw_dir: PathBuf) -> anyhow::Result<()> {
    match cli.command {
        Some(Commands::Health) => {
            let report = openclaw::health::build_health_report(&openclaw_dir)?;
            openclaw::health::print_health_report(&report);
            Ok(())
        }
        Some(Commands::Agents) => {
            let cfg = openclaw::config::read_config(&openclaw_dir)?;
            let agents = openclaw::config::resolve_agents(&cfg, &openclaw_dir);
            println!();
            println!("  Discovered {} agent(s):", agents.len());
            println!();
            for agent in &agents {
                let default_mark = if agent.is_default { " (default)" } else { "" };
                let name = agent.name.as_deref().unwrap_or("-");
                let model = agent.model.as_deref().unwrap_or("-");
                let ws_exists = if agent.workspace_path.exists() { "✅" } else { "❌" };
                println!("  {} {} [{}]{}", ws_exists, agent.id, name, default_mark);
                println!("     Workspace: {}", agent.workspace_path.display());
                println!("     Model:     {}", model);
                println!("     Sessions:  {}", agent.sessions_dir.display());
                println!();
            }
            Ok(())
        }
        Some(Commands::Version) => {
            println!("clawborg v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None => {
            // ── Foreground server mode ──
            // Banner was already printed in main(). Show animated startup steps.

            // Step 1: Read OpenClaw config
            let cfg_result = openclaw::config::read_config(&openclaw_dir);
            match &cfg_result {
                Ok(_) => ui::startup_step_ok("Reading OpenClaw config", ""),
                Err(e) => {
                    ui::startup_step_err("Reading OpenClaw config", &e.to_string());
                    return Err(anyhow::anyhow!("{}", e));
                }
            }
            let cfg = cfg_result.unwrap();

            // Step 2: Discover agents
            let agents = openclaw::config::resolve_agents(&cfg, &openclaw_dir);
            ui::startup_step_ok(
                "Loading agents",
                &format!("{} discovered", agents.len()),
            );

            // Steps 3-4 ("Building session cache", "Starting file watcher") are shown
            // inside server::run() at the actual execution points.
            run_server(cli.port, openclaw_dir, cli.no_watch, cli.readonly, None, true).await
        }
        // These are handled synchronously before the runtime starts
        Some(Commands::Start) | Some(Commands::Stop) | Some(Commands::Log { .. }) => {
            unreachable!("start/stop/log handled before async runtime")
        }
    }
}
