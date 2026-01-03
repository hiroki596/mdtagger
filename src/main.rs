use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde_yaml::Value;
use std::fs;
use std::path::PathBuf;

/// MarkdownファイルのFront Matterにタグを追加するCLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// 対象のMarkdownファイル
    #[arg(value_name = "FILE")]
    path: PathBuf,

    /// 追加するタグ（複数指定可）
    #[arg(value_name = "TAGS", num_args = 1..)]
    tags: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = &cli.path;

    // 1. ファイルを読み込む
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {:?}", path))?;

    // 2. Front Matter と 本文 を分離する
    // 正規表現: 先頭の --- ブロックをキャプチャする
    // (?s) は . が改行にもマッチするようにするフラグ
    let re = Regex::new(r"(?s)^---\n(.*?)\n---\n(.*)").unwrap();
    
    let (mut yaml_val, body) = if let Some(caps) = re.captures(&content) {
        // Front Matterがある場合
        let yaml_str = caps.get(1).unwrap().as_str();
        let body_str = caps.get(2).unwrap().as_str();
        let val: Value = serde_yaml::from_str(yaml_str)?;
        (val, body_str.to_string())
    } else {
        // Front Matterがない場合、新規作成
        (serde_yaml::Value::Mapping(serde_yaml::Mapping::new()), content)
    };

    // 3. タグを更新する
    update_tags(&mut yaml_val, &cli.tags)?;

    // 4. ファイルに書き戻す
    let new_yaml = serde_yaml::to_string(&yaml_val)?;
    // serde_yamlは先頭に "---" を付け、末尾は改行のみなので、整形して結合
    let new_content = format!("---\n{}---\n{}", new_yaml, body);

    fs::write(path, new_content)
        .with_context(|| format!("Failed to write file: {:?}", path))?;

    println!("Updated {:?}: Added tags {:?}", path, cli.tags);

    Ok(())
}

fn update_tags(yaml_val: &mut Value, new_tags: &[String]) -> Result<()> {
    // YAMLがオブジェクト(Mapping)でない場合はエラーまたは初期化
    let mapping = yaml_val.as_mapping_mut().context("Invalid Front Matter structure")?;

    // "tags" フィールドを取得、なければ作成
    let tags_key = Value::String("tags".to_string());
    
    // tagsフィールドが存在しない場合は空配列で初期化
    if !mapping.contains_key(&tags_key) {
        mapping.insert(tags_key.clone(), Value::Sequence(Vec::new()));
    }

    let tags_val = mapping.get_mut(&tags_key).unwrap();

    // tags が "tag1" のように文字列の場合、["tag1"] に変換する
    if tags_val.is_string() {
        let s = tags_val.as_str().unwrap().to_string();
        *tags_val = Value::Sequence(vec![Value::String(s)]);
    }

    // ここで tags_val は必ず Sequence (配列) であるはず
    if let Some(seq) = tags_val.as_sequence_mut() {
        // 既存のタグをStringのVectorとして取得
        let mut current_strings: Vec<String> = seq.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        // 新しいタグを追加
        for tag in new_tags {
            current_strings.push(tag.clone());
        }

        // 重複排除 (順序保持のため retain など使うか、単純に dedup)
        // ここでは単純化のため sort + dedup する（順序が変わるのが嫌なら IndexSet 等を使う）
        current_strings.sort();
        current_strings.dedup();

        // YAMLのValueに戻す
        *seq = current_strings.into_iter().map(Value::String).collect();
    } else {
        // tagsがあるけど配列でも文字列でもない（nullなど）場合
        return Err(anyhow::anyhow!("'tags' field is not a list or string"));
    }

    Ok(())
}