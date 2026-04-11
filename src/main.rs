mod acp;
mod config;
mod discord;
mod error_display;
mod format;
mod reactions;
mod setup;

use clap::{Parser, Subcommand};
use serenity::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[derive(Parser)]
#[command(name = "openab")]
#[command(about = "OpenAB - Discord Bot for AI Agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 互動式設定精靈，產生 config.toml
    Setup,
    /// 執行 Bot（預設）
    Run {
        /// 設定檔路徑（預設: config.toml）
        config: Option<std::path::PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Setup) => {
            setup::run_setup()?;
            return Ok(());
        }
        Some(Commands::Run { config }) => {
            start_bot(config).await?;
        }
        None => {
            start_bot(None).await?;
        }
    }
    Ok(())
}

async fn start_bot(config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "openab=info".into()),
        )
        .init();

    let config_path = config_path
        .unwrap_or_else(|| PathBuf::from("config.toml"));

    let cfg = config::load_config(&config_path)?;
    info!(
        agent_cmd = %cfg.agent.command,
        pool_max = cfg.pool.max_sessions,
        channels = ?cfg.discord.allowed_channels,
        users = ?cfg.discord.allowed_users,
        reactions = cfg.reactions.enabled,
        "config loaded"
    );

    let pool = Arc::new(acp::SessionPool::new(cfg.agent, cfg.pool.max_sessions));
    let ttl_secs = cfg.pool.session_ttl_hours * 3600;

    let allowed_channels = parse_id_set(&cfg.discord.allowed_channels, "allowed_channels")?;
    let allowed_users = parse_id_set(&cfg.discord.allowed_users, "allowed_users")?;
    info!(channels = allowed_channels.len(), users = allowed_users.len(), "parsed allowlists");

    let handler = discord::Handler {
        pool: pool.clone(),
        allowed_channels,
        allowed_users,
        reactions_config: cfg.reactions,
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&cfg.discord.bot_token, intents)
        .event_handler(handler)
        .await?;

    // Spawn cleanup task
    let cleanup_pool = pool.clone();
    let cleanup_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            cleanup_pool.cleanup_idle(ttl_secs).await;
        }
    });

    // Run bot until SIGINT/SIGTERM
    let shard_manager = client.shard_manager.clone();
    let shutdown_pool = pool.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("shutdown signal received");
        shard_manager.shutdown_all().await;
    });

    info!("starting discord bot");
    client.start().await?;

    // Cleanup
    cleanup_handle.abort();
    shutdown_pool.shutdown().await;
    info!("openab shut down");
    Ok(())
}

fn parse_id_set(raw: &[String], label: &str) -> anyhow::Result<HashSet<u64>> {
    let set: HashSet<u64> = raw
        .iter()
        .filter_map(|s| match s.parse() {
            Ok(id) => Some(id),
            Err(_) => {
                tracing::warn!(value = %s, label = label, "ignoring invalid entry");
                None
            }
        })
        .collect();
    if !raw.is_empty() && set.is_empty() {
        anyhow::bail!("all {label} entries failed to parse — refusing to start with an empty allowlist");
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_no_args_parses() {
        let result = Cli::try_parse_from(["openab"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_setup_command() {
        let result = Cli::try_parse_from(["openab", "setup"]);
        assert!(result.is_ok());
        match result.unwrap().command {
            Some(Commands::Setup) => {}
            other => panic!("expected Setup, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_run_command_no_config() {
        let result = Cli::try_parse_from(["openab", "run"]);
        assert!(result.is_ok());
        match result.unwrap().command {
            Some(Commands::Run { config }) => assert!(config.is_none()),
            other => panic!("expected Run, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_run_command_with_config() {
        let result = Cli::try_parse_from(["openab", "run", "/path/to/config.toml"]);
        assert!(result.is_ok());
        match result.unwrap().command {
            Some(Commands::Run { config }) => {
                assert_eq!(config.unwrap(), PathBuf::from("/path/to/config.toml"));
            }
            other => panic!("expected Run, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_help_flag() {
        let result = Cli::try_parse_from(["openab", "--help"]);
        // --help causes parse error with TryParse
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_id_set_valid_ids() {
        let ids = vec!["123".to_string(), "456".to_string()];
        let set = parse_id_set(&ids, "test").unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&123));
        assert!(set.contains(&456));
    }

    #[test]
    fn test_parse_id_set_mixed_valid_invalid() {
        let ids = vec!["123".to_string(), "invalid".to_string(), "456".to_string()];
        let set = parse_id_set(&ids, "test").unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&123));
        assert!(set.contains(&456));
    }

    #[test]
    fn test_parse_id_set_all_invalid_fails() {
        let ids = vec!["invalid1".to_string(), "invalid2".to_string()];
        let result = parse_id_set(&ids, "my_label");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("my_label"));
    }

    #[test]
    fn test_parse_id_set_empty_ok() {
        let ids: Vec<String> = vec![];
        let set = parse_id_set(&ids, "test").unwrap();
        assert!(set.is_empty());
    }
}
