use std::io::{self, Write};
use serde::Serialize;

const VALID_AGENTS: [&str; 3] = ["claude", "kiro", "codex"];

/// Top-level config structure for config.toml
#[derive(Serialize)]
struct Config {
    discord: DiscordConfig,
    agent: AgentConfig,
    pool: PoolConfig,
    reactions: ReactionsConfig,
}

#[derive(Serialize)]
struct DiscordConfig {
    bot_token: String,
    allowed_channels: Vec<String>,
}

#[derive(Serialize)]
struct AgentConfig {
    command: String,
    args: Vec<String>,
    working_dir: String,
}

#[derive(Serialize)]
struct PoolConfig {
    max_sessions: u32,
    session_ttl_hours: u32,
}

#[derive(Serialize)]
struct ReactionsConfig {
    enabled: bool,
    remove_after_reply: bool,
    emojis: EmojisConfig,
    timing: TimingConfig,
}

#[derive(Serialize)]
struct EmojisConfig {
    queued: String,
    thinking: String,
    tool: String,
    coding: String,
    web: String,
    done: String,
    error: String,
}

#[derive(Serialize)]
struct TimingConfig {
    debounce_ms: u32,
    stall_soft_ms: u32,
    stall_hard_ms: u32,
    done_hold_ms: u32,
    error_hold_ms: u32,
}

/// Check if config.toml exists at the given path
pub fn config_file_exists(path: &std::path::Path) -> bool {
    path.exists()
}

/// Prompt user for overwrite confirmation, returns true to overwrite
pub fn prompt_overwrite() -> anyhow::Result<bool> {
    print!("config.toml already exists. Overwrite? (y/N): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Validate bot_token for TOML safety using allowlist (ASCII alphanumeric, dash, period, underscore)
pub fn validate_bot_token(token: &str) -> anyhow::Result<()> {
    if token.is_empty() {
        anyhow::bail!("Token cannot be empty");
    }
    if !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_') {
        anyhow::bail!("Token must only contain ASCII letters, numbers, dashes, periods, or underscores");
    }
    Ok(())
}

/// Validate agent command against known valid values
pub fn validate_agent_command(cmd: &str) -> anyhow::Result<()> {
    if !VALID_AGENTS.contains(&cmd) {
        anyhow::bail!(
            "Agent command must be one of: {}",
            VALID_AGENTS.join(", ")
        );
    }
    Ok(())
}

/// Validate channel ID is numeric
pub fn validate_channel_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty() {
        anyhow::bail!("Channel ID cannot be empty");
    }
    if !id.chars().all(|c| c.is_ascii_digit()) {
        anyhow::bail!("Channel ID must be numeric only");
    }
    Ok(())
}

/// Generate config.toml content from provided values using proper TOML serialization
pub fn generate_config(bot_token: &str, agent_command: &str, channel_ids: Vec<String>) -> String {
    let config = Config {
        discord: DiscordConfig {
            bot_token: bot_token.to_string(),
            allowed_channels: channel_ids,
        },
        agent: AgentConfig {
            command: agent_command.to_string(),
            args: vec!["acp".to_string(), "--trust-all-tools".to_string()],
            working_dir: "/home/agent".to_string(),
        },
        pool: PoolConfig {
            max_sessions: 10,
            session_ttl_hours: 24,
        },
        reactions: ReactionsConfig {
            enabled: true,
            remove_after_reply: false,
            emojis: EmojisConfig {
                queued: "👀".to_string(),
                thinking: "🤔".to_string(),
                tool: "🔥".to_string(),
                coding: "👨💻".to_string(),
                web: "⚡".to_string(),
                done: "🆗".to_string(),
                error: "😱".to_string(),
            },
            timing: TimingConfig {
                debounce_ms: 700,
                stall_soft_ms: 10000,
                stall_hard_ms: 30000,
                done_hold_ms: 1500,
                error_hold_ms: 2500,
            },
        },
    };
    toml::to_string_pretty(&config).expect("config serialization failed")
}

/// Interactive setup wizard
pub fn run_setup() -> anyhow::Result<()> {
    println!();
    println!("  🤖 OpenAB Interactive Setup Wizard");
    println!();

    // Check for existing config
    if config_file_exists(std::path::Path::new("config.toml")) {
        if !prompt_overwrite()? {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // 1. Bot Token
    print!("? Bot Token: ");
    io::stdout().flush()?;
    let bot_token = rpassword::read_password()?;
    if let Err(e) = validate_bot_token(&bot_token) {
        anyhow::bail!("Invalid token: {}", e);
    }

    // 2. Agent Command
    print!("? Agent command (claude/kiro/codex) [claude]: ");
    io::stdout().flush()?;
    let mut agent_command = String::new();
    io::stdin().read_line(&mut agent_command)?;
    let agent_command = agent_command.trim();
    let agent_command = if agent_command.is_empty() {
        "claude"
    } else {
        agent_command
    };
    if let Err(e) = validate_agent_command(agent_command) {
        anyhow::bail!("{}", e);
    }

    // 3. Channel ID
    print!("? Allowed channel ID: ");
    io::stdout().flush()?;
    let mut channel_id = String::new();
    io::stdin().read_line(&mut channel_id)?;
    let channel_id = channel_id.trim();
    if let Err(e) = validate_channel_id(channel_id) {
        anyhow::bail!("{}", e);
    }

    // Generate and write config
    let config_content = generate_config(&bot_token, agent_command, vec![channel_id.to_string()]);
    std::fs::write("config.toml", &config_content)
        .map_err(|e| anyhow::anyhow!("Failed to write config.toml: {}", e))?;

    println!();
    println!("✅ config.toml generated!");
    println!();
    println!("Run with:");
    println!("  openab              # uses config.toml");
    println!("  openab run [path]   # uses custom config");
    println!();
    println!("Tip: You can add more channel IDs to allowed_channels in config.toml");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_bot_token_ok() {
        assert!(validate_bot_token("sk-ant-token123").is_ok());
        assert!(validate_bot_token("simple_token").is_ok());
        assert!(validate_bot_token("token.with-dashes_123").is_ok());
    }

    #[test]
    fn test_validate_bot_token_rejects_invalid_chars() {
        // Quotes are rejected (not in allowlist)
        let result = validate_bot_token("token\"with\"quote");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ASCII"));

        // Newlines are rejected
        let result = validate_bot_token("token\nwith\nnewline");
        assert!(result.is_err());

        // Tab is rejected
        let result = validate_bot_token("token\twith\ttab");
        assert!(result.is_err());

        // Space is rejected
        let result = validate_bot_token("token with space");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_bot_token_rejects_empty() {
        let result = validate_bot_token("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_agent_command_valid() {
        for agent in VALID_AGENTS {
            assert!(validate_agent_command(agent).is_ok());
        }
    }

    #[test]
    fn test_validate_agent_command_invalid() {
        let result = validate_agent_command("invalid");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("claude"));
    }

    #[test]
    fn test_validate_channel_id_ok() {
        assert!(validate_channel_id("1492329565824094370").is_ok());
        assert!(validate_channel_id("123").is_ok());
    }

    #[test]
    fn test_validate_channel_id_empty() {
        let result = validate_channel_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_channel_id_non_numeric() {
        let result = validate_channel_id("abc123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("numeric"));
    }

    #[test]
    fn test_generate_config_basic() {
        let config = generate_config("my_token", "claude", vec!["1492329565824094370".to_string()]);

        assert!(config.contains(r#"bot_token = "my_token""#));
        assert!(config.contains(r#"allowed_channels = ["1492329565824094370"]"#));
        assert!(config.contains(r#"command = "claude""#));
    }

    #[test]
    fn test_generate_config_kiro_agent() {
        let config = generate_config("token123", "kiro", vec!["999888777".to_string()]);

        assert!(config.contains(r#"bot_token = "token123""#));
        assert!(config.contains(r#"allowed_channels = ["999888777"]"#));
        assert!(config.contains(r#"command = "kiro""#));
    }

    #[test]
    fn test_generate_config_contains_required_sections() {
        let config = generate_config("t", "c", vec!["1".to_string()]);

        // Discord section
        assert!(config.contains("[discord]"));
        assert!(config.contains("bot_token"));
        assert!(config.contains("allowed_channels"));

        // Agent section
        assert!(config.contains("[agent]"));
        assert!(config.contains("command"));
        assert!(config.contains("acp"));
        assert!(config.contains("--trust-all-tools"));
        assert!(config.contains(r#"working_dir = "/home/agent""#));

        // Pool section
        assert!(config.contains("[pool]"));
        assert!(config.contains("max_sessions = 10"));
        assert!(config.contains("session_ttl_hours = 24"));

        // Reactions section
        assert!(config.contains("[reactions]"));
        assert!(config.contains("enabled = true"));
        assert!(config.contains("[reactions.emojis]"));
        assert!(config.contains("[reactions.timing]"));
    }

    #[test]
    fn test_generate_config_no_placeholder_leftover() {
        let config = generate_config("token", "agent", vec!["channel".to_string()]);

        assert!(!config.contains("{bot_token}"));
        assert!(!config.contains("{agent_command}"));
        assert!(!config.contains("{allowed_channels}"));
    }

    #[test]
    fn test_generate_config_special_characters_in_token() {
        // Tokens may contain special chars (but not quotes or newlines)
        let config = generate_config("sk-ant...shes", "claude", vec!["123".to_string()]);

        assert!(config.contains(r#"bot_token = "sk-ant...shes""#));
    }

    #[test]
    fn test_config_file_exists_when_file_present() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        std::fs::write(&config_path, "test").unwrap();
        assert!(config_file_exists(&config_path));
    }

    #[test]
    fn test_config_file_exists_when_file_absent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        assert!(!config_file_exists(&config_path));
    }

    #[test]
    fn test_prompt_overwrite_logic() {
        // y and Y return true, everything else returns false
        fn check_overwrite(input: &str) -> bool {
            input.trim().eq_ignore_ascii_case("y")
        }

        assert!(check_overwrite("y"));
        assert!(check_overwrite("Y"));
        assert!(check_overwrite("y\n"));
        assert!(check_overwrite(" y "));
        assert!(!check_overwrite("n"));
        assert!(!check_overwrite("N"));
        assert!(!check_overwrite(""));
        assert!(!check_overwrite("yes"));
        assert!(!check_overwrite("yy"));
    }
}
