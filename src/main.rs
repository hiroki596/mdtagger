use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::{Confirm, Select};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};
use strsim::levenshtein;

// --- データ構造 (変更なし) ---
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TagEntry {
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TagConfig {
    tags: Vec<TagEntry>,
}

// --- CLI引数定義 (ここを変更) ---
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(value_name = "FILE")]
    path: PathBuf,

    #[arg(value_name = "TAGS", num_args = 1..)]
    tags: Vec<String>,

    /// タグデータベースのパスを指定 (環境変数 SMART_TAGS_DB でも設定可)
    #[arg(
        long,
        value_name = "DB_PATH", 
        env = "SMART_TAGS_DB",      // 環境変数を読みに行く
        default_value = "tags_db.json" // デフォルトはカレントディレクトリ
    )]
    db: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let md_path = &cli.path;
    let db_path = &cli.db; // 引数からパスを取得

    // 1. 指定されたパスからロード
    let mut config = load_config(db_path)?;

    let mut resolved_tags = Vec::new();
    let mut config_updated = false;

    println!("Using DB: {:?}", db_path); // 現在どのDBを使っているか表示

    println!("Checking tags...");
    for raw_tag in &cli.tags {
        let (final_tag, updated) = resolve_tag(raw_tag, &mut config)?;
        resolved_tags.push(final_tag);
        if updated {
            config_updated = true;
        }
    }

    // 2. 指定されたパスへ保存
    if config_updated {
        save_config(db_path, &config)?;
        println!("✨ Tag database updated at {:?}", db_path);
    }

    // 3. Markdownファイルを更新
    update_markdown(md_path, &resolved_tags)?;

    println!(
        "✅ Successfully added tags to {:?}: {:?}",
        md_path, resolved_tags
    );
    Ok(())
}

// --- ロジック: タグ解決 (変更なし) ---
fn resolve_tag(input: &str, config: &mut TagConfig) -> Result<(String, bool)> {
    // 省略 (前回のコードと同じ)
    // A. 完全一致
    for entry in &config.tags {
        if entry.name == input || entry.aliases.iter().any(|a| a == input) {
            if entry.name != input {
                println!("   Mapping '{}' -> '{}'", input, entry.name);
            }
            return Ok((entry.name.clone(), false));
        }
    }
    // B. あいまい検索
    let suggestions: Vec<(usize, usize)> = config
        .tags
        .iter()
        .enumerate()
        .map(|(i, t)| (i, levenshtein(&t.name, input)))
        .filter(|(_, dist)| *dist <= 3)
        .collect();

    if !suggestions.is_empty() {
        println!("Tag '{}' is unknown.", input);
        let mut selections = Vec::new();
        for (idx, _dist) in &suggestions {
            let tag_name = &config.tags[*idx].name;
            selections.push(format!("Use existing '{}' (Typo correction)", tag_name));
        }
        let best_match_idx = suggestions[0].0;
        let best_match_name = config.tags[best_match_idx].name.clone();
        selections.push(format!(
            "Register '{}' as alias for '{}'",
            input, best_match_name
        ));
        selections.push(format!("Create new tag '{}'", input));

        let selection = Select::new()
            .with_prompt("How to handle this?")
            .items(&selections)
            .default(0)
            .interact()?;

        if selection < suggestions.len() {
            let target_idx = suggestions[selection].0;
            return Ok((config.tags[target_idx].name.clone(), false));
        } else if selection == suggestions.len() {
            config.tags[best_match_idx].aliases.push(input.to_string());
            return Ok((best_match_name.clone(), true));
        }
    }

    // C. 新規登録
    let confirm = Confirm::new()
        .with_prompt(format!("Register new tag '{}' to database?", input))
        .default(true)
        .interact()?;

    if confirm {
        config.tags.push(TagEntry {
            name: input.to_string(),
            aliases: Vec::new(),
        });
        Ok((input.to_string(), true))
    } else {
        Ok((input.to_string(), false))
    }
}

// --- I/O周りの修正: PathBufを受け取るように変更 ---

fn load_config(path: &Path) -> Result<TagConfig> {
    if !path.exists() {
        return Ok(TagConfig::default());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read DB file: {:?}", path))?;
    let config = serde_json::from_str(&content).unwrap_or_default();
    Ok(config)
}

fn save_config(path: &Path, config: &TagConfig) -> Result<()> {
    // 親ディレクトリが存在しない場合は作成する（親切設計）
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content).with_context(|| format!("Failed to write DB file: {:?}", path))?;
    Ok(())
}

// --- Markdown更新 (変更なし) ---
fn update_markdown(path: &PathBuf, new_tags: &[String]) -> Result<()> {
    // 省略 (前回のコードと同じ)
    let content = fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;

    let re = Regex::new(r"(?s)^---\n(.*?)\n---\n(.*)").unwrap();

    let (mut yaml_val, body) = if let Some(caps) = re.captures(&content) {
        let yaml_str = caps.get(1).unwrap().as_str();
        let body_str = caps.get(2).unwrap().as_str();
        let val: Value =
            serde_yaml::from_str(yaml_str).unwrap_or(Value::Mapping(serde_yaml::Mapping::new()));
        (val, body_str.to_string())
    } else {
        (Value::Mapping(serde_yaml::Mapping::new()), content)
    };

    let mapping = yaml_val.as_mapping_mut().context("Invalid Front Matter")?;
    let tags_key = Value::String("tags".to_string());

    if !mapping.contains_key(&tags_key) {
        mapping.insert(tags_key.clone(), Value::Sequence(Vec::new()));
    }

    let tags_val = mapping.get_mut(&tags_key).unwrap();

    if tags_val.is_string() {
        let s = tags_val.as_str().unwrap().to_string();
        *tags_val = Value::Sequence(vec![Value::String(s)]);
    }

    if let Some(seq) = tags_val.as_sequence_mut() {
        let mut current_strings: Vec<String> = seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        for tag in new_tags {
            current_strings.push(tag.clone());
        }
        current_strings.sort();
        current_strings.dedup();
        *seq = current_strings.into_iter().map(Value::String).collect();
    }

    let new_yaml = serde_yaml::to_string(&yaml_val)?;
    let new_content = format!("---\n{}---\n{}", new_yaml, body);
    fs::write(path, new_content)?;

    Ok(())
}
