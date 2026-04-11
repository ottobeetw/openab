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
    println!("  🤖 OpenAB 互動設定精靈");
    println!();

    // 1. Bot Token
    print!("? Bot Token: ");
    io::stdout().flush()?;
    let bot_token = rpassword::read_password()?;
    if bot_token.trim().is_empty() {
        anyhow::bail!("Bot Token 不能為空白");
    }

    // 2. Agent Command
    println!();
    println!("? Agent 命令 (claude/kiro/codex) [claude]: ");
    print!("  (直接 Enter 使用預設值: claude) ");
    io::stdout().flush()?;
    let mut agent_command = String::new();
    io::stdin().read_line(&mut agent_command)?;
    let agent_command = agent_command.trim();
    let agent_command = if agent_command.is_empty() { "claude" } else { agent_command };

    // 3. Channel ID
    println!();
    print!("? 允許頻道 ID: ");
    io::stdout().flush()?;
    let mut channel_id = String::new();
    io::stdin().read_line(&mut channel_id)?;
    let channel_id = channel_id.trim();
    if channel_id.is_empty() {
        anyhow::bail!("頻道 ID 不能為空白");
    }

    // Generate and write config
    let config_content = generate_config(&bot_token, agent_command, &channel_id);
    std::fs::write("config.toml", &config_content)
        .map_err(|e| anyhow::anyhow!("寫入 config.toml 失敗: {}", e))?;

    println!();
    println!("✅ config.toml 已產生！");
    println!();
    println!("執行方式:");
    println!("  openab              # 使用 config.toml");
    println!("  openab [路徑]       # 使用指定設定檔");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Token 可能包含特殊字元
        let config = generate_config("sk-ant-token-with-dashes", "claude", "123");

        assert!(config.contains(r#"bot_token = "sk-ant-token-with-dashes""#));
    }
}
