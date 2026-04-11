use std::io::{self, Write};

const CONFIG_TEMPLATE: &str = r#"[discord]
bot_token = "{bot_token}"
allowed_channels = [{allowed_channels}]

[agent]
command = "{agent_command}"
args = ["acp", "--trust-all-tools"]
working_dir = "/home/agent"

[pool]
max_sessions = 10
session_ttl_hours = 24

[reactions]
enabled = true
remove_after_reply = false

[reactions.emojis]
queued = "👀"
thinking = "🤔"
tool = "🔥"
coding = "👨💻"
web = "⚡"
done = "🆗"
error = "😱"

[reactions.timing]
debounce_ms = 700
stall_soft_ms = 10000
stall_hard_ms = 30000
done_hold_ms = 1500
error_hold_ms = 2500
"#;

const VALID_AGENTS: [&str; 3] = ["claude", "kiro", "codex"];

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

/// Validate bot_token for TOML safety (rejects quotes and newlines)
pub fn validate_bot_token(token: &str) -> anyhow::Result<()> {
    if token.contains('"') {
        anyhow::bail!("Token must not contain quote characters");
    }
    if token.contains('\n') {
        anyhow::bail!("Token must not contain newline characters");
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

/// Generate config.toml content from provided values
pub fn generate_config(bot_token: &str, agent_command: &str, channel_id: &str) -> String {
    CONFIG_TEMPLATE
        .replace("{bot_token}", bot_token)
        .replace("{allowed_channels}", &format!("\"{}\"", channel_id))
        .replace("{agent_command}", agent_command)
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
        anyhow::bail!("Invalid token format: {}", e);
    }
    if bot_token.trim().is_empty() {
        anyhow::bail!("Bot Token cannot be empty");
    }

    // 2. Agent Command
    print!("? Agent command (claude/kiro/codex) [claude]: ");
    io::stdout().flush()?;
    let mut agent_command = String::new();
    io::stdin().read_line(&mut agent_command)?;
    let agent_command = agent_command.trim();
    let agent_command = if agent_command.is_empty() { "claude" } else { agent_command };
    if let Err(e) = validate_agent_command(agent_command) {
        anyhow::bail!("{}", e);
    }

    // 3. Channel ID
    println!();
    print!("? Allowed channel ID: ");
    io::stdout().flush()?;
    let mut channel_id = String::new();
    io::stdin().read_line(&mut channel_id)?;
    let channel_id = channel_id.trim();
    if let Err(e) = validate_channel_id(channel_id) {
        anyhow::bail!("{}", e);
    }

    // Generate and write config
    let config_content = generate_config(&bot_token, agent_command, &channel_id);
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
    }

    #[test]
    fn test_validate_bot_token_rejects_quote() {
        let result = validate_bot_token("token\"with\"quote");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quote"));
    }

    #[test]
    fn test_validate_bot_token_rejects_newline() {
        let result = validate_bot_token("token\nwith\nnewline");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("newline"));
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
        let config = generate_config("my_token", "claude", "1492329565824094370");

        assert!(config.contains(r#"bot_token = "my_token""#));
        assert!(config.contains(r#"allowed_channels = ["1492329565824094370"]"#));
        assert!(config.contains(r#"command = "claude""#));
    }

    #[test]
    fn test_generate_config_kiro_agent() {
        let config = generate_config("token123", "kiro", "999888777");

        assert!(config.contains(r#"bot_token = "token123""#));
        assert!(config.contains(r#"allowed_channels = ["999888777"]"#));
        assert!(config.contains(r#"command = "kiro""#));
    }

    #[test]
    fn test_generate_config_contains_required_sections() {
        let config = generate_config("t", "c", "1");

        // Discord section
        assert!(config.contains("[discord]"));
        assert!(config.contains("bot_token"));
        assert!(config.contains("allowed_channels"));

        // Agent section
        assert!(config.contains("[agent]"));
        assert!(config.contains("command"));
        assert!(config.contains(r#"args = ["acp", "--trust-all-tools"]"#));
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
        let config = generate_config("token", "agent", "channel");

        assert!(!config.contains("{bot_token}"));
        assert!(!config.contains("{agent_command}"));
        assert!(!config.contains("{allowed_channels}"));
    }

    #[test]
    fn test_generate_config_special_characters_in_token() {
        // Tokens may contain special chars (but not quotes or newlines)
        let config = generate_config("sk-ant...shes", "claude", "123");

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
