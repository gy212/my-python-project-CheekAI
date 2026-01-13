# CheekAI Rust + Tauri 重构实施计划

## 项目概览

将 CheekAI 从 Python + Electron 技术栈迁移至 Rust + Tauri，分 6 个阶段实施。

---

## 阶段 0：环境准备 ✅ 已完成

### 目标
搭建 Rust + Tauri 开发环境，初始化项目骨架

### 任务清单
- [x] 安装 Rust 工具链 (rustup) - Rust 1.91.1
- [x] 安装 Tauri CLI - Tauri CLI 2.9.5
- [x] 安装 Node.js 依赖 - Node.js 24.3.0, npm 11.4.2
- [x] 初始化 Tauri 项目 (`npm create tauri-app`)
- [x] 选择前端框架 - Vue 3 + TypeScript
- [x] 配置开发环境
- [x] 验证 `cargo tauri dev` 能正常启动

### 验收标准
- [x] 运行 `cargo tauri dev` 显示 Tauri 窗口
- [x] 前端热重载正常工作
- [x] Rust 代码能编译通过

### 预计产出
```
cheekAI/
├── src-tauri/
│   ├── src/main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.vue (或 .tsx/.svelte)
│   └── main.ts
├── package.json
└── vite.config.ts
```

---

## 阶段 1：数据模型层 ✅ 已完成

### 目标
将 Python Pydantic 模型迁移为 Rust 结构体

### 任务清单
- [x] 创建 `src-tauri/src/models/mod.rs`
- [x] 迁移请求模型
  - [x] `DetectRequest` → Rust struct
  - [x] `BatchDetectRequest` → Rust struct
  - [x] `PaperAnalyzeRequest` → Rust struct
- [x] 迁移响应模型
  - [x] `DetectResponse` → Rust struct
  - [x] `SegmentResponse` → Rust struct
  - [x] `AggregationResponse` → Rust struct
- [x] 迁移配置模型
  - [x] `ApiConfig` → Rust struct
  - [x] `HistoryItem` → Rust struct
- [x] 为所有结构体实现 `Serialize`/`Deserialize`
- [x] 编写单元测试验证序列化

### 验收标准
- [x] `cargo test` 模型测试全部通过
- [x] JSON 序列化/反序列化与原 Python 格式兼容

### 参考文件
- `legacy-python-electron/backend/app/schemas.py`

---

## 阶段 2：核心服务层 ✅ 已完成

### 目标
实现核心业务逻辑（不含 AI 调用）

### 任务清单

#### 2.1 文本处理服务
- [x] 创建 `src-tauri/src/services/text_processor.rs`
- [x] 实现标点符号规范化 (中英文) - `normalize_punctuation()`
- [x] 实现 Token 估算 (中文字符 + 英文单词) - `estimate_tokens()`
- [x] 实现句子分割 (带偏移量追踪) - `split_sentences_advanced()`
- [x] 实现段落块构建 - `build_paragraph_blocks()`
- [x] 实现句子块构建 - `build_sentence_blocks()`
- [x] 实现风格计量分析 - `compute_stylometry()`

#### 2.2 文档预处理服务
- [ ] 创建 `src-tauri/src/services/preprocess.rs` (待后续实现)
- [ ] 实现 PDF/DOCX/TXT 解析 (待后续实现)

#### 2.3 配置存储服务
- [x] 创建 `src-tauri/src/services/config_store.rs`
- [x] 实现配置文件读写 (JSON) - `ConfigStore::load()/save()`
- [x] 实现版本备份机制 - `create_backup()`
- [x] 实现 API Key 安全存储 - `get_api_key()/set_api_key()`

### 验收标准
- [x] 文本处理单元测试通过 (6 tests)
- [ ] 能正确解析 PDF/DOCX/TXT 示例文件 (待后续实现)
- [x] 配置读写与版本备份正常

### 参考文件
- `legacy-python-electron/backend/app/service.py`
- `legacy-python-electron/backend/app/preprocess.py`
- `legacy-python-electron/backend/app/config_store.py`

---

## 阶段 3：AI 提供商集成 ✅ 已完成

### 目标
实现 GLM 和 Deepseek API 调用

### 任务清单
- [x] 创建 `src-tauri/src/services/providers.rs`
- [x] 实现 HTTP 客户端封装 (`reqwest`) - `ProviderClient`
- [x] 实现 GLM API 调用 - `call_glm()`
  - [x] 认证头处理 (Bearer token)
  - [x] 请求构建 (ChatRequest)
  - [x] 响应解析 (ChatResponse)
  - [x] 错误处理 (ProviderError)
- [x] 实现 Deepseek API 调用 - `call_deepseek()`
- [x] 实现代理支持 - `ProviderClient::with_proxy()`
- [x] 实现重试机制 (reasoning 模式失败时自动重试)
- [x] 实现超时处理 (80s timeout)
- [x] 实现 API Key 获取 - `get_api_key()`

### 验收标准
- [x] GLM API 调用接口已实现
- [x] Deepseek API 调用接口已实现
- [x] 代理配置支持
- [x] 错误处理完善 (8 tests passed)

### 参考文件
- `legacy-python-electron/backend/app/providers.py`

---

## 阶段 4：检测逻辑实现 ✅ 已完成

### 目标
实现完整的 AI 文本检测流程

### 任务清单

#### 4.1 检测核心
- [x] 创建 `src-tauri/src/services/detection.rs`
- [x] 实现文本分块 - `build_segments()`
- [x] 实现段落��分析 - `build_paragraph_blocks()`
- [x] 实现句子级分析 - `build_sentence_blocks()`
- [x] 实现信号聚合 - `SegmentSignals` (LLM/困惑度/风格)

#### 4.2 结果聚合
- [x] 实现阈值判定 - `AggregationThresholds` (0.65/0.75/0.85/0.90)
- [x] 实现决策推导 - `derive_decision()` (pass/review/flag)
- [x] 实现置信度计算 - `aggregate_segments()`
- [x] 实现对比度锐化 - `contrast_sharpen_segments()`

#### 4.3 高级功能
- [x] 实现双模式检测 - `dual_mode_detect()`
- [x] 实现一致性检查 - `compare_dual_mode_results()`
- [ ] 实现校准功能 (待后续实现)
- [ ] 实现多轮分析 (待后续实现)

### 验收标准
- [x] 检测逻辑单元测试通过 (11 tests)
- [x] 双模式检测结果合理
- [ ] 批量检测正常工作 (待阶段5实现)
- [ ] 性能测试 (待阶段7实现)

### 参考文件
- `legacy-python-electron/backend/app/service.py`
- `legacy-python-electron/backend/app/services/response_builder.py`

---

## 阶段 5：Tauri 命令暴露 ✅ 已完成

### 目标
将 Rust 服务暴露为 Tauri 命令供前端调用

### 任务清单

#### 5.1 检测命令
- [x] 创建 `src-tauri/src/api/detect.rs`
- [x] `detect_text` - 单文本检测
- [x] `detect_dual_mode` - 双模式检测
- [ ] `detect_batch` - 批量检测 (待后续实现)
- [ ] `analyze_paper` - 论文分析 (待后续实现)

#### 5.2 配置命令
- [x] 创建 `src-tauri/src/api/config.rs`
- [x] `get_config` - 获取配置
- [x] `save_config` - 保存配置
- [x] `get_providers` - 获取可用提供商

#### 5.3 凭证管理
- [x] 集成 `keyring` crate
- [x] `store_api_key` - 安全存储 API Key
- [x] `get_api_key` - 获取 API Key
- [x] `delete_api_key` - 删除 API Key

#### 5.4 历史/文件命令 (待后续实现)
- [ ] 历史记录管理
- [ ] 文件上传处理
- [ ] 导出结果

### 验收标准
- [x] 核心命令已注册到 Tauri (12 tests passed)
- [x] 错误信息正确传递到前端
- [x] API Key 安全存储在系统凭��管理器

### 参考文件
- `legacy-python-electron/backend/app/routers/*.py`
- `legacy-python-electron/desktop/main.js`

---

## 阶段 6：前端迁移 ✅ 核心完成

### 目标
将 Electron 前端迁移到 Tauri 前端

### 任务清单

#### 6.1 基础 UI
- [x] 实现基础布局 (双栏布局)
- [ ] 实现无边框窗口 (待后续优化)
- [ ] 实现自定义标题栏 (待后续优化)
- [ ] 实现窗口拖拽 (待后续优化)

#### 6.2 核心功能页面
- [x] 实现检测结果展示
  - [x] 段落级结果
  - [x] 双模式对比结果
  - [x] 概率颜色指示
- [x] 实现进度指示 (Loading Overlay)
- [x] 实现错误提示 (alert)
- [ ] 实现文件拖放上传区域 (待后续实现)

#### 6.3 配置页面
- [x] 实现 API Key 配置界面
- [x] 实现敏感度选择
- [x] 实现双模式检测开关
- [ ] 实现提供商选择 (待后续实现)
- [ ] 实现代理配置 (待后续实现)

#### 6.4 历史记录 (待后续实现)
- [ ] 实现历史列表
- [ ] 实现历史详情查看
- [ ] 实现历史删除

#### 6.5 导出功能
- [x] 实现 JSON 导出
- [x] 实现 CSV 导出

#### 6.6 样式迁移
- [x] 迁移 CSS 样式
- [x] 保持中文 UI 文案
- [x] 响应式适配

### 验收标准
- [x] 核心 UI 功能正常
- [x] 检测功能正常工作
- [x] 导出功能正常
- [ ] 文件拖放正常 (待后续实现)

### 参考文件
- `legacy-python-electron/desktop/renderer/index.html`
- `legacy-python-electron/desktop/renderer/index.js`
- `legacy-python-electron/desktop/renderer/style.css`

---

## 阶段 7：测试与发布 ✅ 待开始

### 目标
完成测试、优化和发布准备

### 任务清单

#### 7.1 测试
- [ ] 编写集成测试
- [ ] 端到端功能测试
- [ ] 性能对比测试
- [ ] 跨平台测试 (Windows 优先)

#### 7.2 优化
- [ ] 性能分析与优化
- [ ] 内存使用优化
- [ ] 启动速度优化
- [ ] 打包体积优化

#### 7.3 发布准备
- [ ] 配置 `tauri.conf.json` 发布设置
- [ ] 配置应用图标
- [ ] 配置安装程序 (NSIS)
- [ ] 编写更新日志
- [ ] 更新 README.md

#### 7.4 构建发布
- [ ] `cargo tauri build` 生成安装包
- [ ] 测试安装包
- [ ] 准备发布

### 验收标准
- [ ] 所有测试通过
- [ ] 打包体积 < 20MB (对比 Electron ~150MB)
- [ ] 启动速度 < 2s
- [ ] 安装包能正常安装和运行

---

## 进度追踪

| 阶段 | 状态 | 开始日期 | 完成日期 |
|------|------|----------|----------|
| 阶段 0: 环境准备 | ✅ 已完成 | 2025-12-10 | 2025-12-10 |
| 阶段 1: 数据模型层 | ✅ 已完成 | 2025-12-10 | 2025-12-10 |
| 阶段 2: 核心服务层 | ✅ 已完成 | 2025-12-10 | 2025-12-10 |
| 阶段 3: AI 提供商集成 | ✅ 已完成 | 2025-12-10 | 2025-12-10 |
| 阶段 4: 检测逻辑实现 | ✅ 已完成 | 2025-12-10 | 2025-12-10 |
| 阶段 5: Tauri 命令暴露 | ✅ 已完成 | 2025-12-10 | 2025-12-10 |
| 阶段 6: 前端迁移 | ✅ 核心完成 | 2025-12-10 | 2025-12-10 |
| 阶段 7: 测试与发布 | ⏳ 待开始 | - | - |

---

## 风险与注意事项

1. **PDF 解析兼容性** - Rust PDF 库可能与 Python 版本解析结果有差异，需要充分测试
2. **中文处理** - 确保使用 `unicode-segmentation` 正确处理中文分词
3. **API 兼容性** - 保持与原 GLM/Deepseek API 调用格式一致
4. **凭证迁移** - 考虑从旧版本迁移已保存的 API Key
5. **配置迁移** - 提供旧配置文件迁移工具

---

## 快速命令参考

```bash
# 开发
cargo tauri dev

# 构建
cargo tauri build

# 测试
cargo test

# 代码检查
cargo clippy

# 格式化
cargo fmt
```
