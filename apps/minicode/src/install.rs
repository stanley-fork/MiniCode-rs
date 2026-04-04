use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;

use crate::config::{MiniCodeSettings, save_minicode_settings};

fn prompt_line(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut stdout = io::stdout();
    if let Some(d) = default {
        write!(stdout, "{} [{}]: ", prompt, d)?;
    } else {
        write!(stdout, "{}: ", prompt)?;
    }
    stdout.flush()?;

    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let value = buf.trim().to_string();
    if !value.is_empty() {
        Ok(value)
    } else if let Some(d) = default {
        Ok(d.to_string())
    } else {
        Err(anyhow::anyhow!("Input cannot be empty"))
    }
}

fn has_path_entry(target: &str) -> bool {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|p| p == target)
}

pub fn run_install_wizard() -> Result<()> {
    println!("mini-code installer");

    let settings_path = crate::config::mini_code_settings_path();
    println!("Configuration will be written to: {}", settings_path.display());
    println!("Settings are stored separately and won't affect other local tool configurations.");
    println!();

    let model_default = std::env::var("ANTHROPIC_MODEL").ok();
    let base_url_default = std::env::var("ANTHROPIC_BASE_URL")
        .ok()
        .or_else(|| Some("https://api.anthropic.com".to_string()));
    let auth_token_default = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();

    let model = prompt_line("Model name", model_default.as_deref())?;
    let base_url = prompt_line(
        "ANTHROPIC_BASE_URL",
        base_url_default.as_deref(),
    )?;

    // Prompt for auth token with secret handling
    let saved_token_suffix = if auth_token_default.is_some() {
        " [saved]"
    } else {
        " [not set]"
    };
    let mut stdout = io::stdout();
    write!(stdout, "ANTHROPIC_AUTH_TOKEN{}: ", saved_token_suffix)?;
    stdout.flush()?;
    let mut token_input = String::new();
    io::stdin().read_line(&mut token_input)?;
    let auth_token = token_input.trim();
    let auth_token = if !auth_token.is_empty() {
        auth_token.to_string()
    } else if let Some(saved) = &auth_token_default {
        saved.clone()
    } else {
        return Err(anyhow::anyhow!("ANTHROPIC_AUTH_TOKEN cannot be empty"));
    };

    let mut env = std::collections::HashMap::new();
    env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(base_url.clone()),
    );
    env.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        serde_json::Value::String(auth_token.clone()),
    );
    env.insert(
        "ANTHROPIC_MODEL".to_string(),
        serde_json::Value::String(model.clone()),
    );

    save_minicode_settings(MiniCodeSettings {
        model: Some(model.clone()),
        env: Some(env),
        ..MiniCodeSettings::default()
    })?;

    println!();
    println!("Installation complete.");
    println!("Configuration file: {}", settings_path.display());
    println!("You can now start MiniCode from the TUI.");

    // For Rust build, suggest adding to PATH if needed
    let target_bin = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".local")
        .join("bin");

    if !has_path_entry(target_bin.to_string_lossy().as_ref()) {
        println!();
        println!(
            "Note: {} is not in your PATH.",
            target_bin.display()
        );
        println!("You can add it to ~/.bashrc or ~/.zshrc:");
        println!("export PATH=\"{}:$PATH\"", target_bin.display());
    }

    Ok(())
}
