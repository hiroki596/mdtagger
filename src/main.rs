use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::{Confirm, Select};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};
use strsim::levenshtein;

// --- 定数: タグ辞書ファイルのパス (簡易的にカレントディレクトリとしています) ---
const DB_FILENAME: &str = "tags_db.json";

// --- データ構造: JSON辞書 ---
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

// --- CLI引数定義 ---
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(value_name = "FILE")]
    path: PathBuf,

    #[arg(value_name = "TAGS", num_args = 1..)]
    tags: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let md_path = &cli.path;

    // 1. タグ辞書 (JSON) をロード
    let mut config = load_config(DB_FILENAME)?;

    // 2. 入力されたタグを1つずつ解決（正規化 or 学習）
    let mut resolved_tags = Vec::new();
    let mut config_updated = false;

    println!("Checking tags...");
    for raw_tag in &cli.tags {
        let (final_tag, updated) = resolve_tag(raw_tag, &mut config)?;
        resolved_tags.push(final_tag);
        if updated {
            config_updated = true;
        }
    }

    // 3. 辞書に変更があれば保存
    if config_updated {
        save_config(DB_FILENAME, &config)?;
        println!("✨ Tag database updated.");
    }

    // 4. Markdownファイルを更新
    update_markdown(md_path, &resolved_tags)?;

    println!("✅ Successfully added tags to {:?}: {:?}", md_path, resolved_tags);
    Ok(())
}

// --- ロジック: タグ解決・対話 ---
fn resolve_tag(input: &str, config: &mut TagConfig) -> Result<(String, bool)> {
    // A. 完全一致 (名前 or エイリアス) チェック
    for entry in &config.tags {
        if entry.name == input || entry.aliases.iter().any(|a| a == input) {
            // 既に知っているタグなら、正規名(name)を返す
            if entry.name != input {
                println!("   Mapping '{}' -> '{}'", input, entry.name);
            }
            return Ok((entry.name.clone(), false));
        }
    }

    // B. あいまい検索 (類似度判定)
    // 距離が 3 以下のものを候補とする
    let suggestions: Vec<(usize, usize)> = config.tags.iter().enumerate()
        .map(|(i, t)| (i, levenshtein(&t.name, input)))
        .filter(|(_, dist)| *dist <= 3) // 閾値: 3文字以内の違い
        .collect();

    // 候補が見つかった場合、ユーザーに聞く
    if !suggestions.is_empty() {
        println!("Tag '{}' is unknown.", input);
        
        let mut selections = Vec::new();
        // 選択肢の作成
        for (idx, _dist) in &suggestions {
            let tag_name = &config.tags[*idx].name;
            selections.push(format!("Use existing '{}' (Typo correction)", tag_name));
        }
        // エイリアス登録の選択肢
        // 一番近い候補（先頭）をデフォルトのエイリアス先に提案
        let best_match_idx = suggestions[0].0;
        let best_match_name = config.tags[best_match_idx].name.clone();
        selections.push(format!("Register '{}' as alias for '{}'", input, best_match_name));
        
        // 新規作成
        selections.push(format!("Create new tag '{}'", input));

        let selection = Select::new()
            .with_prompt("How to handle this?")
            .items(&selections)
            .default(0)
            .interact()?;

        if selection < suggestions.len() {
            // 1. 既存タグとして使う (一時的修正)
            let target_idx = suggestions[selection].0;
            return Ok((config.tags[target_idx].name.clone(), false));
        } else if selection == suggestions.len() {
            // 2. エイリアスとして登録
            config.tags[best_match_idx].aliases.push(input.to_string());
            return Ok((best_match_name.clone(), true));
        } else {
            // 3. 新規作成へ
            // 下のフローへ流す
        }
    }

    // C. 新規登録フロー (類似なし or ユーザーが新規選択)
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
        // 登録拒否された場合でもファイルには書く場合
        Ok((input.to_string(), false))
    }
}

// --- ロジック: 設定ファイル I/O ---
fn load_config(path: &str) -> Result<TagConfig> {
    if !Path::new(path).exists() {
        return Ok(TagConfig::default());
    }
    let content = fs::read_to_string(path)?;
    let config = serde_json::from_str(&content).unwrap_or_default();
    Ok(config)
}

fn save_config(path: &str, config: &TagConfig) -> Result<()> {
    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}

// --- ロジック: Markdown更新 (前回と同じ堅牢な実装) ---
fn update_markdown(path: &PathBuf, new_tags: &[String]) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {:?}", path))?;

    let re = Regex::new(r"(?s)^---\n(.*?)\n---\n(.*)").unwrap();
    
    let (mut yaml_val, body) = if let Some(caps) = re.captures(&content) {
        let yaml_str = caps.get(1).unwrap().as_str();
        let body_str = caps.get(2).unwrap().as_str();
        let val: Value = serde_yaml::from_str(yaml_str).unwrap_or(Value::Mapping(serde_yaml::Mapping::new()));
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

    // 文字列なら配列へ昇格
    if tags_val.is_string() {
        let s = tags_val.as_str().unwrap().to_string();
        *tags_val = Value::Sequence(vec![Value::String(s)]);
    }

    if let Some(seq) = tags_val.as_sequence_mut() {
        let mut current_strings: Vec<String> = seq.iter()
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