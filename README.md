# Markdown Tagger
Markdownに以下のようなタグを付けるツール

```markdown
---
tags: [tag1, tag2, tag3, ...]
---
```

## 使い方
ファイルを指定して実行すると，ファイルの先頭にタグを追加します．

```bash
mdt file.md tag1 tag2 tag3
```

検索する場合はgrepなどで`tags:`を検索してください．

```bash
grep -r "tags: \[.*tag1.*\]" .
```
```bash
rg "tags: \[.*tag1.*\]" .
```