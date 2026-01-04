# Markdown Tagger

MarkdownファイルのYAML Front Matterにタグを追加・管理するCLIツールです。
表記ゆれを防ぐための「学習機能（エイリアス管理）」と、対話的なタグ登録機能を備えています。

## ✨ 特徴

* **Front Matterの自動編集**: 既存のメタデータ（タイトルや日付など）を壊さずにタグを追加します。Front Matterがない場合は自動生成します。
* **タグの重複排除**: 同じタグが二重に登録されるのを防ぎます。
* **対話的な学習機能**:
    * **スペルミス修正**: 未知のタグ（例: `dvelop`）が入力された際、類似する既存タグ（`development`）を提案します。
    * **エイリアス登録**: 短縮形（例: `py`）を正式名称（`python`）のエイリアスとしてその場で登録できます。
* **柔軟なDB保存場所**: タグの辞書データ（JSON）の場所を環境変数や引数で指定可能です。

## 🛠 ビルドとインストール

Rust環境（Cargo）が必要です。

### 1. ビルド

```bash
# プロジェクトのルートディレクトリで実行
cargo build --release
```

### 2. インストール（パスを通す）

ビルドされたバイナリをパスの通ったディレクトリにコピーします。

**Mac / Linux:**
```bash
cp target/release/mdtagger /usr/local/bin/
```
※ または、`.zshrc` などで `target/release` にパスを通しても構いません。

**Windows (PowerShell):**
```powershell
copy target\release\mdtagger.exe C:\Windows\System32\
# または任意のPathの通ったフォルダへ
```

## 🚀 使い方

### 基本コマンド

```bash
smart_tags <ファイルパス> <タグ1> <タグ2> ...
```

**例:**
```bash
smart_tags memo.md rust cli
```
→ `memo.md` の `tags` に `rust` と `cli` が追加されます。

### オプション

* `-d`, `--db <PATH>`: タグデータベース（JSON）のパスを一時的に指定します。

```bash
smart_tags memo.md python --db ./my_project_tags.json
```

---

## ⚙️ 設定（タグデータベースの場所）

タグの辞書データ（`tags_db.json`）の保存場所は、以下の優先順位で決定されます。

1.  **コマンドライン引数**: `--db /path/to/db.json`
2.  **環境変数**: `SMART_TAGS_DB`
3.  **デフォルト**: カレントディレクトリの `tags_db.json`

### 推奨設定（環境変数）

常に同じ辞書を使いたい場合（グローバル設定）、シェルの設定ファイルに環境変数を追加することをおすすめします。

**~/.zshrc または ~/.bashrc:**

```bash
# 自分のホームディレクトリ配下にDBを置く例
export SMART_TAGS_DB="$HOME/.config/smart_tags/db.json"
```

設定後、シェルを再読み込みするかターミナルを再起動してください。
※ ディレクトリが存在しない場合、初回実行時に自動作成されます。

---

## 🤖 対話モードの例

未知のタグを入力した際、ツールは以下のように振る舞います。

**入力:**
```bash
smart_tags note.md rs
```
（`rs` は未登録、`rust` は登録済みの場合）

**対話画面:**
```text
Tag 'rs' is unknown.
How to handle this?
> Use existing 'rust' (Typo correction)        # 今回だけ 'rust' に直す
  Register 'rs' as alias for 'rust'            # 今後 'rs' と打てば 'rust' になる
  Create new tag 'rs'                          # 新しいタグとして登録
```

ここで「Register 'rs' as alias...」を選ぶと、次回からは `rs` と入力するだけで自動的に `rust` として記録されます。

## 📦 依存ライブラリ

* `clap`: 引数解析
* `serde`, `serde_json`, `serde_yaml`: データシリアライズ
* `dialoguer`: 対話的UI
* `strsim`: 文字列類似度計算（レーベンシュタイン距離）
* `anyhow`: エラーハンドリング
* `regex`: Front Matter解析