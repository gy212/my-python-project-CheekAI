# CheekAI

CheekAI 是一个 AI 生成文本检测桌面应用：前端使用 Vue 3 + TypeScript，后端使用 Rust（Tauri）。支持导入 TXT / PDF / DOCX，并基于“段落 + 句子”双模式进行交叉检测与融合。

> 说明：仓库内的 `legacy-python-electron/` 为旧版 Python/FastAPI + Electron 代码存档，仅供参考，不再作为主实现。

## 架构

- 前端：`src/`（Vue 3 + TS）
- 后端：`src-tauri/`（Rust + Tauri）
- 核心检测逻辑：`src-tauri/src/services/detection/`
  - `segment_builder` / `aggregation` / `comparison` / `dual_mode` / `llm_analyzer`
- 文本处理与分句/分段：`src-tauri/src/services/text_processor.rs`、`src-tauri/src/services/sentence_segmenter.rs`

## 双模式检测（你关心的“工作逻辑/限制”）

双模式指：同时跑“段落侧”和“句子侧”，并做对比与融合：
- 段落侧：按段落块检测，得到整体概率与段落级证据
- 句子侧：先做正文过滤，再按句子块检测（更细粒度）
- 对比：`compare_dual_mode_results` 计算一致性/差异（阈值目前为 `0.20`）
- 融合：`fuse_aggregations` 按权重融合（段落 `0.6` + 句子 `0.4`）

关键限制/约束（用于对齐与性能）：
- 所有块（段落/句子）都使用 **UTF-8 字节偏移** `start/end` 对齐原文，避免多字节字符截断。
- 句子侧会先做“非正文段落过滤”（规则 + LLM），避免目录/参考文献/致谢等被拆成句子后浪费检测额度。
- 分句“LLM 归整”只会**合并边界**，不会重写文本，确保 offsets 永远能回贴到原文。

实现入口（可直接打开看）：
- `src-tauri/src/services/detection/dual_mode.rs`
- `src-tauri/src/services/detection/content_filter.rs`
- `src-tauri/src/services/sentence_segmenter.rs`

## 快速开始（开发）

环境要求：
- Node.js 16+（建议 18+）
- Rust toolchain（含 cargo）
- Windows 下建议安装 WebView2（Tauri 依赖）

启动（前后端热更新）：
```bash
npm install
npm run tauri dev
```

仅检查 Rust：
```bash
cd src-tauri
cargo check
```

构建安装包：
```bash
npm run tauri build
```

## AI Provider 配置

你可以在应用设置页配置 API Key；后端也支持从环境变量读取（优先级更高）：
- OpenAI：`OPENAI_API_KEY` / `CHEEKAI_OPENAI_API_KEY`
- Gemini：`GEMINI_API_KEY` / `CHEEKAI_GEMINI_API_KEY`
- DeepSeek：`DEEPSEEK_API_KEY` / `CHEEKAI_DEEPSEEK_API_KEY`
- GLM（智谱）：`GLM_API_KEY` / `CHEEKAI_GLM_API_KEY`
- Anthropic/Claude：`ANTHROPIC_API_KEY` / `CHEEKAI_ANTHROPIC_API_KEY`

## 智能分句（可选服务 + LLM 归整）

句子分割入口：`src-tauri/src/services/sentence_segmenter.rs` 的 `build_sentence_blocks_smart`：
1. 优先调用本机分句服务（默认 `http://127.0.0.1:8788`）
2. 服务不可用则回退到本地规则分句
3. 可选：LLM 归整（只合并“疑似误切”边界；不改写文本）

关闭句子侧 LLM 归整：
- 环境变量：`CHEEKAI_DISABLE_SENTENCE_LLM_REFINE=1`

分句服务端实现（Python，spaCy + wtpsplit）：`legacy-python-electron/services/sentence_segmenter/server.py`

更多细节：`docs/sentence_segmentation.md`

## 调试：用你的 DOCX 测试分句/过滤

新增了一个调试二进制（输出分句与块信息，便于定位“误切/漏正文/耗时”）：
```bash
cd src-tauri
cargo run -p cheekAI --bin segment_docx -- "C:\\Users\\21240\\Desktop\\文档\\数字化视角下南充体育非物质文化遗产保护与发展策略研究 V2.0  .docx" --filter --llm --provider openai --out segment_docx_result.json
```

## 更多文档

- `docs/PROJECT_DOCUMENTATION.md`
- `docs/detection_algorithm.md`
- `docs/MIGRATION_RUST_TAURI.md`

## 许可证

MIT License，见 `LICENSE`。
