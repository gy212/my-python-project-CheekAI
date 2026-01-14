# 句子分割（智能分句 + LLM 归整）

本项目在双模式检测的“句子侧”使用更稳健的分句策略，目标是减少引号/缩写/省略号/括号等导致的误切，同时避免对 UTF-8 多字节字符造成任何截断。

## 当前实现

入口函数：`src-tauri/src/services/sentence_segmenter.rs` 中的 `build_sentence_blocks_smart`

流程（按优先级）：
1. **spaCy 分句服务**（可选）：尝试调用 `http://127.0.0.1:8788` 获取句子 offsets（UTF-8 byte offsets）。
2. **本地规则回退**：服务不可用时使用本地 `split_sentences_advanced` 分句。
3. **LLM 归整（可选）**：仅对“疑似误切”的边界发起 LLM 裁决，输出“需要合并的边界 index 列表”，然后在原始 offsets 上做合并（不重写文本）。
4. **合块**：按 `target_chars/max_chars` 将句子聚合为 `TextBlock(label="sentence_block")`。

## 配置与开关

- 关闭 LLM 归整：设置环境变量 `CHEEKAI_DISABLE_SENTENCE_LLM_REFINE=1`
  - 仍会使用 spaCy（若可用）/本地规则分句，只是不再调用 LLM 做边界纠错。

## spaCy 分句服务（可选）

项目里保留了一个可复用的 Python 服务端实现：`legacy-python-electron/services/sentence_segmenter/server.py`（默认端口 `8788`）。

当服务可用时，Rust 侧会自动使用它；服务不可用时会自动回退，不影响检测流程（只是分句质量可能下降）。

