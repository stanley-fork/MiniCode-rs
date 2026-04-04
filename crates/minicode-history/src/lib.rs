use std::fs;

use anyhow::Result;
use minicode_config::mini_code_history_path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct HistoryFile {
    entries: Vec<String>,
}

/// 加载历史输入并限制最多保留最近 200 条。
pub fn load_history_entries() -> Vec<String> {
    let path = mini_code_history_path();
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(parsed) = serde_json::from_str::<HistoryFile>(&content) else {
        return vec![];
    };
    let keep = 200usize;
    if parsed.entries.len() <= keep {
        return parsed.entries;
    }
    parsed.entries[parsed.entries.len() - keep..].to_vec()
}

/// 保存历史输入并仅写入最近 200 条记录。
pub fn save_history_entries(entries: &[String]) -> Result<()> {
    let path = mini_code_history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let keep = 200usize;
    let slice = if entries.len() <= keep {
        entries.to_vec()
    } else {
        entries[entries.len() - keep..].to_vec()
    };
    let payload = HistoryFile { entries: slice };
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&payload)?),
    )?;
    Ok(())
}
