# Sentence Segmenter Service

独立的 spaCy 分句微服务，供 Rust Tauri 调用。

## 安装

```bash
cd services/sentence_segmenter
pip install -r requirements.txt
python -m spacy download zh_core_web_sm  # 中文模型
python -m spacy download en_core_web_sm  # 英文模型（可选）
```

## 启动服务

```bash
python server.py --port 8788
```

## API

### 健康检查
```
GET /health
Response: {"status": "ok", "service": "sentence_segmenter"}
```

### 分句
```
POST /segment
Body: {"text": "这是第一句。这是第二句！", "language": "zh"}
Response: {
  "sentences": [
    {"text": "这是第一句。", "start": 0, "end": 7},
    {"text": "这是第二句！", "start": 7, "end": 14}
  ]
}
```

### 分句并聚合为块
```
POST /segment/blocks
Body: {
  "text": "...",
  "language": "zh",
  "minChars": 50,
  "targetChars": 200,
  "maxChars": 300
}
Response: {
  "blocks": [
    {
      "index": 0,
      "label": "sentence_block",
      "needDetect": true,
      "start": 0,
      "end": 200,
      "text": "...",
      "sentenceCount": 3
    }
  ]
}
```

## 与 Rust Tauri 集成

Rust 端会自动检测服务是否可用：
- 如果可用：使用 spaCy 智能分句
- 如果不可用：回退到本地规则分句

使用示例（Rust）：
```rust
use crate::services::sentence_segmenter::build_sentence_blocks_spacy;

let blocks = build_sentence_blocks_spacy(text, "zh", 50, 200, 300).await;
```
