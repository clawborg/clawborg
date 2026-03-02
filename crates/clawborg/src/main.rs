use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod openclaw;
mod routes;
mod server;
mod types;
mod watcher;
mod ws;

#[derive(Parser)]
#[command(name = "clawborg", version, about = "Dashboard for OpenClaw AI agent fleets")]
struct Cli {
    /// Port for the dashboard
    #[arg(short, long, default_value_t = 3104)]
    port: u16,

    /// OpenClaw directory path
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("clawborg=info")),
        )
        .init();

    let cli = Cli::parse();

    // Resolve OpenClaw directory
    let openclaw_dir = cli.dir.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".openclaw")
    });

    if !openclaw_dir.exists() {
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

    match cli.command {
        Some(Commands::Health) => {
            let report = openclaw::health::build_health_report(&openclaw_dir)?;
            openclaw::health::print_health_report(&report);
            Ok(())
        }
        Some(Commands::Agents) => {
            // Print discovered agents
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
            println!("clawborg {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None => {
            // Verify config is readable before starting server
            let cfg = openclaw::config::read_config(&openclaw_dir)?;
            let agents = openclaw::config::resolve_agents(&cfg, &openclaw_dir);

            let config = server::ServerConfig {
                port: cli.port,
                openclaw_dir,
                watch_enabled: !cli.no_watch,
                readonly: cli.readonly,
            };

            println!();
            println!("  🤖 ClawBorg v{}", env!("CARGO_PKG_VERSION"));
            println!("  ─────────────────────────────");
            println!("  Dashboard:  http://localhost:{}", config.port);
            println!("  API:        http://localhost:{}/api", config.port);
            println!("  OpenClaw:   {}", config.openclaw_dir.display());
            println!("  Agents:     {} discovered", agents.len());
            println!(
                "  Mode:       {}",
                if config.readonly { "read-only" } else { "read-write" }
            );
            println!(
                "  Watching:   {}",
                if config.watch_enabled { "on" } else { "off" }
            );
            println!();

            // Print agent summary
            for agent in &agents {
                let name = agent.name.as_deref().unwrap_or(&agent.id);
                let ws_ok = if agent.workspace_path.exists() { "✅" } else { "⚠️" };
                println!("  {} {} → {}", ws_ok, name, agent.workspace_path.display());
            }
            println!();

            server::run(config).await
        }
    }
}
