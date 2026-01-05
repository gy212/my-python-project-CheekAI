# CheekAI 项目深度技术文档

> **版本**: 1.0.0
> **技术栈**: Rust/Tauri 2.x + Vue 3 + TypeScript
> **文档日期**: 2026-01

---

## 目录

1. [项目概述](#1-项目概述)
2. [技术架构总览](#2-技术架构总览)
3. [目录结构详解](#3-目录结构详解)
4. [后端架构 (Rust/Tauri)](#4-后端架构-rusttauri)
5. [前端架构 (Vue 3/TypeScript)](#5-前端架构-vue-3typescript)
6. [核心检测算法](#6-核心检测算法)
7. [数据模型与类型系统](#7-数据模型与类型系统)
8. [API 接口文档](#8-api-接口文档)
9. [配置与环境变量](#9-配置与环境变量)
10. [开发与构建指南](#10-开发与构建指南)

---

## 1. 项目概述

### 1.1 项目定位

CheekAI 是一款专业的 **AI 生成文本检测桌面应用**，旨在帮助用户识别文档中可能由 AI（如 ChatGPT、Claude 等）生成的内容。应用支持多种文档格式（DOCX、TXT、PDF），采用多信号融合的检测算法，结合本地风格分析与云端 LLM 判断，提供高精度的检测结果。

### 1.2 核心功能

| 功能模块 | 描述 |
|---------|------|
| **文本检测** | 支持直接输入文本或上传文件进行 AI 生成内容检测 |
| **双模式检测** | 段落级 + 句子级双重检测，交叉验证提高准确性 |
| **多提供商支持** | 集成 GLM、DeepSeek 等多个 AI 提供商 |
| **灵敏度调节** | 低/中/高三档灵敏度，适应不同场景需求 |
| **结果导出** | 支持 JSON/CSV 格式导出检测结果 |
| **API 密钥管理** | 安全存储 API 密钥（系统凭证管理器） |

### 1.3 技术亮点

- **Rust 后端**: 高性能、内存安全、原生跨平台
- **Tauri 2.x**: 轻量级桌面框架，比 Electron 更小更快
- **Vue 3 Composition API**: 现代化前端架构，逻辑复用性强
- **TypeScript 严格模式**: 全栈类型安全
- **异步并发**: tokio 异步运行时，支持并行 LLM 调用
- **模块化设计**: 检测算法高度模块化，易于扩展和调参

---

## 2. 技术架构总览

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        CheekAI Desktop App                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Frontend (Vue 3)                      │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │    │
│  │  │ TitleBar │ │ Control  │ │TextInput │ │ Results  │   │    │
│  │  │          │ │  Panel   │ │          │ │  Panel   │   │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │              Composables Layer                    │   │    │
│  │  │  useDetection │ useProviders │ useFileHandler    │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                    Tauri IPC (invoke)                           │
│                              │                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   Backend (Rust/Tauri)                   │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │                   API Layer                       │   │    │
│  │  │  detect.rs │ config.rs │ Tauri Commands          │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │                 Services Layer                    │   │    │
│  │  │  ┌────────────────────────────────────────────┐  │   │    │
│  │  │  │            Detection Module                 │  │   │    │
│  │  │  │  segment_builder │ llm_analyzer │ dual_mode │  │   │    │
│  │  │  │  aggregation │ comparison                   │  │   │    │
│  │  │  └────────────────────────────────────────────┘  │   │    │
│  │  │  text_processor │ providers │ config_store       │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │                  Models Layer                     │   │    │
│  │  │  Data Structures │ Serde Serialization           │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                    External Services                             │
│                              │                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  GLM API │ DeepSeek API │ Anthropic API │ OpenAI API    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 数据流向

```
用户输入文本/上传文件
        │
        ▼
┌───────────────────┐
│   文件预处理       │  ← preprocess_file (DOCX/TXT/PDF 解析)
│   normalize_punctuation
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   文本分段         │  ← build_paragraph_blocks / build_sentence_blocks
│   语言检测         │  ← detect_language
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   特征提取         │  ← compute_stylometry (TTR, 重复率, N-gram等)
│   困惑度估计       │  ← estimate_perplexity
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   本地评分         │  ← score_segment_continuous (Logit空间累加)
│   LLM 评分(可选)   │  ← GLM/DeepSeek API 调用
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   结果聚合         │  ← aggregate_segments (置信度加权)
│   决策生成         │  ← derive_decision (pass/review/flag)
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   双模式融合(可选) │  ← fuse_aggregations (段落0.6 + 句子0.4)
│   一致性分析       │  ← compare_dual_mode_results
└───────────────────┘
        │
        ▼
    返回检测结果
```

### 2.3 技术栈详情

| 层级 | 技术 | 版本 | 用途 |
|-----|------|------|------|
| **前端框架** | Vue | 3.5.x | 响应式 UI 框架 |
| **前端语言** | TypeScript | 5.6.x | 类型安全 |
| **构建工具** | Vite | 6.0.x | 快速开发构建 |
| **桌面框架** | Tauri | 2.x | 跨平台桌面应用 |
| **后端语言** | Rust | 2021 Edition | 系统级编程 |
| **异步运行时** | tokio | 1.x | 异步 I/O |
| **HTTP 客户端** | reqwest | 0.12.x | API 调用 |
| **序列化** | serde/serde_json | 1.x | JSON 处理 |
| **日志系统** | tracing | 0.1.x | 结构化日志 |
| **文档解析** | docx-rs, pdf-extract | - | 文件处理 |

---

## 3. 目录结构详解

### 3.1 根目录结构

```
cheekAI/
├── src/                          # Vue 前端源码
├── src-tauri/                    # Rust 后端源码
├── public/                       # 静态资源
├── docs/                         # 项目文档
├── scripts/                      # 开发脚本
├── legacy-python-electron/       # 旧版代码存档
├── index.html                    # 应用入口 HTML
├── package.json                  # 前端依赖配置
├── tsconfig.json                 # TypeScript 配置
├── vite.config.ts                # Vite 构建配置
├── CLAUDE.md                     # Claude Code 工作指南
├── AGENTS.md                     # 开发规范
└── README.md                     # 项目说明
```

### 3.2 前端目录结构 (src/)

```
src/
├── main.ts                       # Vue 应用入口
├── App.vue                       # 根组件
├── vite-env.d.ts                 # Vite 类型声明
├── components/                   # UI 组件
│   ├── index.ts                  # 组件导出索引
│   ├── TitleBar.vue              # 窗口标题栏 (2KB)
│   ├── ControlPanel.vue          # 控制面板 (9KB)
│   ├── TextInput.vue             # 文本输入区 (1.5KB)
│   ├── ResultsPanel.vue          # 结果展示面板 (7.7KB)
│   ├── SettingsModal.vue         # 设置模态框 (19KB)
│   └── LoadingMask.vue           # 加载遮罩 (1.4KB)
├── composables/                  # 组合式函数
│   ├── index.ts                  # 导出索引
│   ├── useDetection.ts           # 检测逻辑 (4.5KB)
│   ├── useProviders.ts           # 提供商管理 (2.9KB)
│   ├── useFileHandler.ts         # 文件处理 (2.7KB)
│   └── useWindow.ts              # 窗口控制 (621B)
├── types/                        # TypeScript 类型定义
│   └── index.ts                  # 所有类型定义 (3.2KB)
└── styles/                       # 样式文件
    └── variables.css             # CSS 变量 (3.7KB)
```

### 3.3 后端目录结构 (src-tauri/)

```
src-tauri/
├── Cargo.toml                    # Rust 项目配置
├── Cargo.lock                    # 依赖锁定
├── tauri.conf.json               # Tauri 应用配置
├── build.rs                      # 构建脚本
├── icons/                        # 应用图标
├── capabilities/                 # Tauri 权限配置
├── logs/                         # 日志文件目录
└── src/                          # Rust 源码
    ├── main.rs                   # 应用入口 (189B)
    ├── lib.rs                    # 库入口/命令注册 (4KB)
    ├── api/                      # API 层 (Tauri 命令)
    │   ├── mod.rs                # 模块导出
    │   ├── detect.rs             # 检测命令 (18KB)
    │   └── config.rs             # 配置命令 (8.6KB)
    ├── services/                 # 服务层
    │   ├── mod.rs                # 模块导出
    │   ├── providers.rs          # AI 提供商集成 (18KB)
    │   ├── text_processor.rs     # 文本处理 (17KB)
    │   ├── sentence_segmenter.rs # 句子分割 (16KB)
    │   ├── config_store.rs       # 配置存储 (7KB)
    │   └── detection/            # 检测核心模块
    │       ├── mod.rs            # 模块导出
    │       ├── segment_builder.rs    # 分段构建 (13KB)
    │       ├── llm_analyzer.rs       # LLM 分析 (24KB)
    │       ├── aggregation.rs        # 结果聚合 (7KB)
    │       ├── comparison.rs         # 双模式对比 (5.8KB)
    │       └── dual_mode.rs          # 双模式检测 (7.3KB)
    └── models/                   # 数据模型
        └── mod.rs                # 所有数据结构 (14KB)
```

---

## 4. 后端架构 (Rust/Tauri)

### 4.1 入口与初始化

#### 4.1.1 应用入口 (`main.rs`)

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cheek_ai_lib::run()
}
```

极简入口，所有逻辑委托给 `lib.rs`。

#### 4.1.2 库入口与命令注册 (`lib.rs`)

**核心职责**:
1. 初始化日志系统（文件 + 控制台双输出）
2. 注册所有 Tauri 命令
3. 配置窗口事件监听

**日志系统配置**:
```rust
fn init_logging() {
    // 日志目录: 开发环境 ./logs, 生产环境 %LOCALAPPDATA%/cheekAI/logs
    let logs_dir = get_logs_dir();

    // 滚动日志文件，保留最近 30 个
    let file_appender = RollingFileAppender::builder()
        .filename_prefix("cheekAI")
        .filename_suffix(".log")
        .max_log_files(30)
        .build(&logs_dir);

    // 双层输出: 文件(无 ANSI) + 控制台(有 ANSI)
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .init();
}
```

**注册的 Tauri 命令**:

| 命令 | 模块 | 功能 |
|-----|------|------|
| `detect_text` | detect.rs | 单模式文本检测 |
| `detect_dual_mode` | detect.rs | 双模式检测 |
| `preprocess_file` | detect.rs | 文件预处理 |
| `get_config` | config.rs | 获取配置 |
| `save_config` | config.rs | 保存配置 |
| `get_providers` | config.rs | 获取可用提供商 |
| `store_api_key` | config.rs | 存储 API 密钥 |
| `get_api_key` | config.rs | 获取 API 密钥 |
| `delete_api_key` | config.rs | 删除 API 密钥 |
| `get_provider_url` | config.rs | 获取提供商 URL |
| `set_provider_url` | config.rs | 设置提供商 URL |
| `diagnose_api_config` | config.rs | API 配置诊断 |
| `test_api_connection` | config.rs | 测试 API 连接 |

### 4.2 API 层详解

#### 4.2.1 检测命令 (`api/detect.rs`)

**并发控制机制**:
```rust
// 全局信号量，防止用户同时启动多个检测任务
static DETECT_GUARD: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn detect_guard() -> &'static Arc<Semaphore> {
    DETECT_GUARD.get_or_init(|| Arc::new(Semaphore::new(1)))
}
```

**`detect_text` 命令流程**:

```rust
#[tauri::command]
pub async fn detect_text(request: DetectTextRequest) -> Result<DetectResponse, String> {
    // 1. 获取信号量，防止并发
    let _guard = detect_guard().clone().try_acquire_owned()
        .map_err(|_| "检测正在进行中，请等待当前检测完成后再试")?;

    // 2. 文本预处理
    let text = normalize_punctuation(&request.text);

    // 3. 语言检测
    let language = request.language.unwrap_or_else(|| detect_language(&text));

    // 4. 构建段落块
    let blocks = build_paragraph_blocks(&text);

    // 5. 分段检测 (LLM 或本地)
    let segments = if provider.is_some() {
        build_segments_with_llm(...).await
    } else {
        build_segments(...)
    };

    // 6. 聚合结果
    let aggregation = aggregate_segments(&segments);

    // 7. 可选: 双模式检测
    let dual_detection = if request.dual_mode {
        Some(dual_mode_detect(...))
    } else {
        None
    };

    Ok(DetectResponse { ... })
}
```

**语言检测算法**:
```rust
fn detect_language(text: &str) -> String {
    let chinese_count = text.chars()
        .filter(|c| *c >= '\u{4e00}' && *c <= '\u{9fff}')
        .count();
    let total_chars = text.chars().filter(|c| !c.is_whitespace()).count();

    // 中文字符占比 > 30% 判定为中文
    if total_chars > 0 && chinese_count as f64 / total_chars as f64 > 0.3 {
        "zh".to_string()
    } else {
        "en".to_string()
    }
}
```

**文件预处理 (`preprocess_file`)**:

支持三种格式:

| 格式 | 处理方式 |
|-----|---------|
| TXT | UTF-8 解码，失败则 lossy UTF-8 |
| DOCX | ZIP 解压 → 解析 document.xml → 过滤噪声段落 → 合并短段 |
| PDF | pdf-extract 库提取文本 → 去空行 |

**DOCX 解析核心逻辑**:
```rust
fn extract_docx_paragraphs_from_document_xml(document_xml: &str) -> Vec<String> {
    // 跳过的标签 (表格、图表、文本框等)
    fn is_skip_tag(name: &str) -> bool {
        matches!(name, "w:tbl" | "w:drawing" | "w:pict" | "w:object" | "w:txbxContent")
    }

    // 轻量级 XML 扫描器
    // 只收集 <w:t> 内的文本
    // 以 </w:p> 为段落边界
}

fn is_noise_paragraph(para: &str) -> bool {
    // 过滤条件:
    // 1. 图表标题: "图1", "Table 2" 等
    // 2. 数字占比 > 60%
    // 3. 极短且无句末标点
    // 4. 字母/中文占比极低 (符号汤)
}
```

#### 4.2.2 配置命令 (`api/config.rs`)

**API 密钥安全存储**:
- 使用系统凭证管理器 (Windows Credential Manager / macOS Keychain)
- 服务名: `cheekAI`
- 用户名: `{provider}_api_key`

**提供商信息结构**:
```rust
#[derive(Serialize)]
pub struct ProviderInfo {
    pub name: String,        // 内部名称: "glm", "deepseek"
    pub display_name: String, // 显示名称: "智谱 GLM", "DeepSeek"
    pub has_key: bool,       // 是否已配置密钥
}
```

### 4.3 服务层详解

#### 4.3.1 AI 提供商服务 (`services/providers.rs`)

**支持的提供商**:

| 提供商 | 默认 URL | 模型 |
|-------|---------|------|
| GLM | `https://open.bigmodel.cn/api/paas/v4/chat/completions` | glm-4-flash |
| DeepSeek | `https://api.deepseek.com/chat/completions` | deepseek-chat, deepseek-reasoner |
| Anthropic | `https://crs.itssx.com/api/v1/messages` | claude-* |
| OpenAI | `https://ai.itssx.com/openai/responses` | gpt-* |
| Gemini | `https://ai.itssx.com/api/v1/chat/completions` | gemini-* |

**ProviderClient 结构**:
```rust
pub struct ProviderClient {
    client: Client,           // reqwest HTTP 客户端
    glm_url: String,
    deepseek_url: String,
    anthropic_url: String,
    openai_responses_url: String,
    gemini_url: String,
}

impl ProviderClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(80))
            .build()
            .unwrap_or_default();
        // URL 可通过环境变量覆盖
    }

    pub fn with_proxy(proxy_url: &str) -> Result<Self, ProviderError> {
        // 支持代理配置
    }
}
```

**API 调用方法**:

| 方法 | 用途 | 特点 |
|-----|------|------|
| `call_glm()` | GLM API 调用 | 支持 JSON 格式响应 |
| `call_deepseek()` | DeepSeek 普通调用 | 不强制 JSON 格式 |
| `call_deepseek_json()` | DeepSeek JSON 调用 | 强制 JSON 格式 |
| `call_anthropic()` | Anthropic API | 合并 system+user 消息 |
| `call_openai_responses()` | OpenAI Responses API | 特殊响应格式 |
| `call_gemini()` | Gemini API | OpenAI 兼容格式 |

**API 密钥获取优先级**:
```rust
pub fn get_api_key(provider: &str) -> Option<String> {
    // 1. 环境变量 (优先)
    //    GLM: GLM_API_KEY, CHEEKAI_GLM_API_KEY
    //    DeepSeek: DEEPSEEK_API_KEY, CHEEKAI_DEEPSEEK_API_KEY

    // 2. 配置文件 (系统凭证管理器)
}
```

#### 4.3.2 文本处理服务 (`services/text_processor.rs`)

**标点规范化 (`normalize_punctuation`)**:
```rust
pub fn normalize_punctuation(text: &str) -> String {
    // 1. 智能引号 → 普通引号: "" → "", '' → ''
    // 2. 破折号规范化: — → -
    // 3. 全角空格 → 半角空格
    // 4. 换行符统一: \r\n, \r → \n
    // 5. 水平空白折叠: 多个空格/Tab → 单空格
    // 6. 每行 trim
}
```

**Token 估算 (`estimate_tokens`)**:
```rust
pub fn estimate_tokens(text: &str) -> i32 {
    // 正则: [A-Za-z0-9_]+ | [\u4e00-\u9fff]
    // 英文按词计数，中文按字计数
    let re = Regex::new(r"[A-Za-z0-9_]+|[\u{4e00}-\u{9fff}]").unwrap();
    re.find_iter(text).count().max(1) as i32
}
```

**段落块构建 (`build_paragraph_blocks`)**:
```rust
pub fn build_paragraph_blocks(text: &str) -> Vec<TextBlock> {
    // 1. 以空行 (\n\n) 分割段落
    // 2. 记录每个段落的 start/end 字节偏移
    // 3. 后处理: 合并短标题块
    //    - is_short_title_like: 非空字符 < 20 且无句末标点
    //    - 连续短标题优先并入后续正文块
}
```

**句子块构建 (`build_sentence_blocks`)**:
```rust
pub fn build_sentence_blocks(
    text: &str,
    min_chars: usize,    // 50
    target_chars: usize, // 200
    max_chars: usize,    // 300
) -> Vec<TextBlock> {
    // 1. split_sentences_advanced 切句
    // 2. 按长度组合:
    //    - 单句 > max_chars: 独立成块
    //    - 累计 <= target_chars: 继续累加
    //    - 累计 > target_chars: 刷新为新块
}
```

**风格计量 (`compute_stylometry`)**:

| 指标 | 计算方式 | AI 特征 |
|-----|---------|--------|
| TTR | 不同词数 / 总词数 | AI 通常更低 (用词模板化) |
| avg_sentence_len | 平均句子字符数 | - |
| function_word_ratio | 功能词占比 | 中文功能词表 |
| repeat_ratio | 出现 ≥3 次的词占比 | AI 通常更高 |
| ngram_repeat_rate | 3-gram 重复率 | AI 通常更高 |
| punctuation_ratio | 标点占字符比 | - |

#### 4.3.3 检测核心模块 (`services/detection/`)

##### 分段构建器 (`segment_builder.rs`)

**连续化评分算法 v2**:

核心思想: 在 Logit 空间累加特征贡献，避免概率"顶死"到边界。

```rust
// Sigmoid 软阈值
fn sigmoid(x: f64, center: f64, k: f64) -> f64 {
    1.0 / (1.0 + ((x - center) / k).exp())
}

// 反向 Sigmoid
fn sigmoid_inv(x: f64, center: f64, k: f64) -> f64 {
    1.0 - sigmoid(x, center, k)
}

// Logit 转概率
fn from_logit(logit: f64) -> f64 {
    1.0 / (1.0 + (-logit).exp())
}
```

**特征贡献表**:

| 特征 | 贡献公式 | 最大贡献 |
|-----|---------|---------|
| TTR 低 | `sigmoid(ttr, 0.58, 0.08) * 1.2` | +1.2 |
| TTR 高 | `sigmoid_inv(ttr, 0.78, 0.06) * (-0.9)` | -0.9 |
| 重复率 | `sigmoid_inv(rep, 0.18, 0.06) * 1.0` | +1.0 |
| N-gram | `sigmoid_inv(ngram, 0.10, 0.04) * 1.1` | +1.1 |
| 句长短 | `sigmoid(avg_len, 35.0, 10.0) * 0.3` | +0.3 |
| 句长长 | `sigmoid_inv(avg_len, 120.0, 25.0) * 0.4` | +0.4 |
| PPL 低 | `sigmoid(ppl, 85.0, 20.0) * 1.0` | +1.0 |
| PPL 高 | `sigmoid_inv(ppl, 200.0, 30.0) * (-0.6)` | -0.6 |

**锚点贡献** (强信号):
- AI 锚点: 低 TTR + 低 PPL + 高重复 → 额外 +1.5
- Human 锚点: 高 TTR + 高 PPL + 低重复 + 合理句长 → 额外 -1.2

**确定性扰动**:
```rust
// 在临界区间 [0.35, 0.75] 应用基于文本 hash 的微小扰动
if prob > 0.35 && prob < 0.75 {
    let noise = deterministic_noise(text, 42) * 0.02;  // ±1% max
    prob = (prob + noise).clamp(0.02, 0.98);
}
```

##### LLM 分析器 (`llm_analyzer.rs`)

**句子长度分流策略**:

| 长度 (字符) | 处理方式 | 模型 |
|------------|---------|------|
| < 10 | 丢弃 | - |
| 10-50 | 本地风格评分 | - |
| 50-300 | DeepSeek LLM | deepseek-chat |
| ≥ 300 | DeepSeek LLM | deepseek-reasoner |

**并发控制参数**:
```rust
const SENTENCE_MIN_LENGTH: usize = 10;
const SENTENCE_LLM_THRESHOLD: usize = 50;
const SENTENCE_REASONER_THRESHOLD: usize = 300;
const DEEPSEEK_SENTENCE_MAX_CONCURRENCY: usize = 4;
const DEEPSEEK_SENTENCE_MAX_ATTEMPTS: usize = 3;
const DEEPSEEK_SENTENCE_TIMEOUT_SECS: u64 = 60;
```

**LLM Prompt 设计**:

单段落检测:
```
你是一个专业的AI文本检测专家。你需要判断给定的文本是否由AI生成。
请分析文本的以下特征：
1. 语言流畅度和自然程度
2. 是否存在AI生成文本的典型特征（如过度正式、缺乏个人风格、重复模式等）
3. 内容的逻辑性和连贯性

请以JSON格式返回结果，包含以下字段：
- probability: 0.000-1.000之间的三位小数
- confidence: 0.000-1.000之间的三位小数
- reasoning: 简短的分析说明
```

批量段落检测 (GLM):
```
请以JSON格式返回结果，包含一个segments数组，每个元素包含：
- chunk_id: 段落编号（从0开始）
- probability: 0.000-1.000之间的三位小数
- confidence: 0.000-1.000之间的三位小数

示例格式：
{"segments": [{"chunk_id": 0, "probability": 0.723, "confidence": 0.856}, ...]}
```

##### 结果聚合 (`aggregation.rs`)

**置信度加权聚合**:
```rust
pub fn aggregate_segments(segments: &[SegmentResponse]) -> AggregationResponse {
    // 权重计算: sqrt(长度) * max(置信度, 0.3)
    let weights: Vec<f64> = segments.iter().map(|s| {
        let len = (s.offsets.end - s.offsets.start).max(50) as f64;
        len.sqrt() * s.confidence.max(0.3)
    }).collect();

    // 加权平均
    let weighted_prob = Σ(ai_probability * weight) / Σ(weight);

    // Trimmed Mean (段落数 ≥5 时)
    // 去掉最高/最低 10% 后取均值

    // 最终结果: 70% 加权 + 30% trimmed
    let overall = 0.7 * weighted_prob + 0.3 * trimmed_prob;
}
```

**决策阈值**:
```rust
impl Default for AggregationThresholds {
    fn default() -> Self {
        Self {
            low: 0.65,
            medium: 0.75,
            high: 0.85,
            very_high: 0.90,
        }
    }
}

// buffer_margin = 0.03
// prob < low - margin → pass
// low - margin ≤ prob < high - margin → review
// prob ≥ high - margin → flag
```

**对比度锐化 (`contrast_sharpen_segments`)**:

用于增强段落间的差异，使结果更具区分度:
```rust
// 1. 计算 Z-score (基于 IQR)
// 2. 根据灵敏度选择 gamma: low=1.10, medium=1.45, high=1.75
// 3. 在 Logit 空间应用锐化
// 4. 二分搜索找到保持均值不变的偏移量
// 5. 低置信度段落混合原值 (80% 原值 + 20% 新值)
```

##### 双模式检测 (`dual_mode.rs`)

**融合权重**:
```rust
const PARAGRAPH_WEIGHT: f64 = 0.6;
const SENTENCE_WEIGHT: f64 = 0.4;
```

**异步并行执行**:
```rust
pub async fn dual_mode_detect_with_llm(...) -> DualDetectionResult {
    // 并行执行段落和句子检测
    let (para_segments, sent_segments) = tokio::join!(
        build_paragraphs_batch_with_glm(...),
        build_sentences_filtered_with_deepseek(...)
    );

    // 分别聚合
    let para_aggregation = aggregate_segments(&para_segments);
    let sent_aggregation = aggregate_segments(&sent_segments);

    // 对比分析
    let comparison = compare_dual_mode_results(...);

    // 融合
    let fused_aggregation = fuse_aggregations(&para_aggregation, &sent_aggregation);
}
```

##### 双模式对比 (`comparison.rs`)

**一致性分析**:
```rust
pub fn compare_dual_mode_results(
    para_segments: &[SegmentResponse],
    sent_segments: &[SegmentResponse],
    text: &str,
    diff_threshold: f64,  // 0.20
) -> ComparisonResult {
    // 1. 计算整体概率差
    // 2. 对所有重叠段落对:
    //    - 覆盖率 > 0.5 才比较
    //    - 方向一致 (>0.5 视为 AI) 计入 consistency_score
    //    - 概率差 > diff_threshold 记录为 divergent_region
}
```

---

## 5. 前端架构 (Vue 3/TypeScript)

### 5.1 应用入口与根组件

#### 5.1.1 入口文件 (`main.ts`)

```typescript
import { createApp } from "vue";
import App from "./App.vue";

createApp(App).mount("#app");
```

#### 5.1.2 根组件 (`App.vue`)

**职责**:
1. 协调所有子组件
2. 管理全局状态
3. 处理组件间通信

**组件结构**:
```vue
<template>
  <TitleBar />
  <div class="app-shell">
    <main class="content-grid">
      <!-- 左栏: 控制面板 + 文本输入 -->
      <section class="column column-controls">
        <ControlPanel ... />
        <TextInput v-model="inputText" />
      </section>

      <!-- 右栏: 结果展示 -->
      <ResultsPanel ... />
    </main>
  </div>
  <LoadingMask :visible="isLoading" />
  <SettingsModal :visible="settingsOpen" />
</template>
```

**Composables 集成**:
```typescript
// 检测逻辑
const {
  inputText, sensitivity, selectedProvider, dualMode,
  isLoading, segments, aggregation, dualResult,
  detect, exportJson, exportCsv
} = useDetection();

// 提供商管理
const { providerOptions, fetchProviders, saveApiKey } = useProviders();

// 文件处理
const { fileName, fileInput, triggerFileSelect, handleFileSelect } = useFileHandler();
```

### 5.2 组件详解

#### 5.2.1 TitleBar.vue (窗口标题栏)

**功能**: 自定义无边框窗口的标题栏

**特性**:
- 可拖拽区域 (data-tauri-drag-region)
- 窗口控制按钮: 最小化、最大化、关闭
- 应用标题显示

**窗口控制实现**:
```typescript
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

function minimize() { appWindow.minimize(); }
function toggleMaximize() { appWindow.toggleMaximize(); }
function close() { appWindow.close(); }
```

#### 5.2.2 ControlPanel.vue (控制面板)

**功能**: 检测参数配置和操作触发

**包含元素**:
| 元素 | 类型 | 功能 |
|-----|------|------|
| 灵敏度选择器 | 下拉框 | 低/中/高三档 |
| 提供商选择器 | 下拉框 | GLM/DeepSeek/本地 |
| 双模式开关 | 复选框 | 启用段落+句子双重检测 |
| 文件上传按钮 | 按钮 | 触发文件选择 |
| 检测按钮 | 按钮 | 启动检测流程 |
| 设置按钮 | 按钮 | 打开 API 密钥配置 |

**Props 定义**:
```typescript
defineProps<{
  sensitivity: string;
  selectedProvider: string;
  providerOptions: ProviderOption[];
  fileName: string;
  dualMode: boolean;
}>();
```

**Events 定义**:
```typescript
defineEmits<{
  (e: "update:sensitivity", value: string): void;
  (e: "update:selectedProvider", value: string): void;
  (e: "update:dualMode", value: boolean): void;
  (e: "detect"): void;
  (e: "open-settings"): void;
  (e: "trigger-file-select"): void;
}>();
```

#### 5.2.3 TextInput.vue (文本输入区)

**功能**: 多行文本输入

**特性**:
- v-model 双向绑定
- 自动调整高度
- 占位符提示

```vue
<template>
  <textarea
    class="text-input"
    :value="modelValue"
    @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
    placeholder="请输入或粘贴待检测文本..."
  />
</template>
```

#### 5.2.4 ResultsPanel.vue (结果展示面板)

**功能**: 展示检测结果

**展示内容**:
1. **整体结果摘要**
   - 整体概率 (带颜色指示)
   - 决策 (pass/review/flag)
   - 置信度

2. **分段结果网格**
   - 每个段落的概率和置信度
   - 颜色编码: ≤30% 绿色, 30-70% 黄色, >70% 红色

3. **双模式对比** (可选)
   - 段落模式概率
   - 句子模式概率
   - 一致性分数

4. **导出按钮**
   - JSON 导出
   - CSV 导出

**概率颜色映射**:
```typescript
function getProbabilityClass(prob: number) {
  if (prob <= 0.30) return "prob-low";      // 绿色
  if (prob < 0.70) return "prob-medium";    // 黄色
  return "prob-high";                        // 红色
}
```

**决策文本映射**:
```typescript
function getDecisionText(decision: string) {
  switch (decision) {
    case "pass": return "通过";
    case "review": return "待审";
    case "flag": return "标记";
    default: return decision;
  }
}
```

#### 5.2.5 SettingsModal.vue (设置模态框)

**功能**: API 密钥配置

**支持的提供商**:
- 智谱 GLM
- DeepSeek

**密钥管理操作**:
- 查看已配置状态
- 输入/更新密钥
- 删除密钥
- 测试连接

**安全特性**:
- 密钥输入框使用 password 类型
- 密钥存储在系统凭证管理器

#### 5.2.6 LoadingMask.vue (加载遮罩)

**功能**: 检测进行中的遮罩层

```vue
<template>
  <div v-if="visible" class="loading-mask">
    <div class="loading-content">
      <div class="spinner"></div>
      <p>{{ text }}</p>
    </div>
  </div>
</template>
```

### 5.3 Composables 详解

#### 5.3.1 useDetection.ts (检测逻辑)

**状态管理**:
```typescript
const inputText = ref("");
const sensitivity = ref("medium");
const selectedProvider = ref("");
const dualMode = ref(false);
const isLoading = ref(false);
const loadingText = ref("正在检测...");
const segments = ref<SegmentResponse[]>([]);
const aggregation = ref<AggregationResponse | null>(null);
const dualResult = ref<DualDetectionResult | null>(null);
```

**计算属性**:
```typescript
const hasResult = computed(() => segments.value.length > 0);
const overallDecision = computed(() => aggregation.value?.decision || "");
const overallProbability = computed(() =>
  aggregation.value ? (aggregation.value.overallProbability * 100).toFixed(1) : "0"
);
```

**检测方法**:
```typescript
async function detect() {
  if (isLoading.value) return;
  if (!inputText.value.trim()) {
    alert("请输入待检测文本");
    return;
  }

  isLoading.value = true;

  try {
    const cmd = dualMode.value ? "detect_dual_mode" : "detect_text";
    const request: DetectTextRequest = {
      text: inputText.value,
      usePerplexity: true,
      useStylometry: true,
      sensitivity: sensitivity.value,
      provider: selectedProvider.value || null,
      dualMode: dualMode.value,
    };

    const result = await invoke(cmd, { request });
    // 处理结果...
  } catch (err) {
    alert("检测失败: " + err);
  } finally {
    isLoading.value = false;
  }
}
```

**导出方法**:
```typescript
function exportJson() {
  const data = { aggregation, segments, dualDetection: dualResult };
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
  // 下载...
}

function exportCsv() {
  const rows = [["段落ID", "AI概率", "置信度", "决策"]];
  segments.value.forEach(seg => {
    rows.push([seg.chunkId, `${(seg.aiProbability * 100).toFixed(1)}%`, ...]);
  });
  // 下载...
}
```

#### 5.3.2 useProviders.ts (提供商管理)

**状态**:
```typescript
const providerOptions = ref<ProviderOption[]>([]);
```

**方法**:
```typescript
async function fetchProviders() {
  const providers = await invoke<ProviderInfo[]>("get_providers");
  providerOptions.value = providers.map(p => ({
    value: p.name,
    label: p.display_name + (p.has_key ? " ✓" : ""),
  }));
}

async function saveApiKey(provider: string, key: string) {
  await invoke("store_api_key", { provider, key });
  await fetchProviders(); // 刷新状态
}
```

#### 5.3.3 useFileHandler.ts (文件处理)

**状态**:
```typescript
const fileName = ref("");
const fileInput = ref<HTMLInputElement | null>(null);
```

**方法**:
```typescript
function triggerFileSelect() {
  fileInput.value?.click();
}

async function handleFileSelect(event: Event, onTextExtracted: (text: string) => void) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

  fileName.value = file.name;

  // 读取文件为 ArrayBuffer
  const arrayBuffer = await file.arrayBuffer();
  const fileData = Array.from(new Uint8Array(arrayBuffer));

  // 调用后端预处理
  const text = await invoke<string>("preprocess_file", {
    fileName: file.name,
    fileData,
  });

  onTextExtracted(text);
}
```

#### 5.3.4 useWindow.ts (窗口控制)

```typescript
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useWindow() {
  const appWindow = getCurrentWindow();

  return {
    minimize: () => appWindow.minimize(),
    toggleMaximize: () => appWindow.toggleMaximize(),
    close: () => appWindow.close(),
  };
}
```

### 5.4 类型定义 (`types/index.ts`)

**核心类型**:

```typescript
// 提供商
interface ProviderInfo {
  name: string;
  display_name: string;
  has_key: boolean;
}

// 灵敏度选项
const SENSITIVITY_OPTIONS = [
  { value: "low", label: "低敏感" },
  { value: "medium", label: "中敏感" },
  { value: "high", label: "高敏感" },
];

// 分段信号
interface SegmentSignals {
  llm_judgment: { prob: number | null; models: string[] };
  perplexity: { ppl: number | null; z: number | null };
  stylometry: {
    ttr: number;
    avg_sentence_len: number;
    function_word_ratio: number | null;
    repeat_ratio: number | null;
    punctuation_ratio: number | null;
  };
}

// 分段响应
interface SegmentResponse {
  chunkId: number;
  language: string;
  offsets: { start: number; end: number };  // UTF-8 字节偏移
  aiProbability: number;
  confidence: number;
  signals: SegmentSignals;
  explanations: string[];
}

// 聚合响应
interface AggregationResponse {
  overallProbability: number;
  overallConfidence: number;
  method: string;
  thresholds: { low: number; medium: number; high: number; veryHigh: number };
  rubricVersion: string;
  decision: string;
  bufferMargin: number;
  stylometryProbability: number | null;
  qualityScoreNormalized: number | null;
}

// 双模式结果
interface DualDetectionResult {
  paragraph: ModeDetectionResult;
  sentence: ModeDetectionResult;
  comparison: ComparisonResult;
  fusedAggregation?: AggregationResponse;
}

// 检测请求
interface DetectTextRequest {
  text: string;
  usePerplexity: boolean;
  useStylometry: boolean;
  sensitivity: string;
  provider: string | null;
  dualMode: boolean;
}
```

### 5.5 样式系统 (`styles/variables.css`)

**CSS 变量定义**:

```css
:root {
  /* 颜色主题 */
  --primary: #6366f1;
  --secondary: #f1f5f9;
  --bg-main: #ffffff;
  --bg-surface: #f8fafc;
  --bg-input: #ffffff;
  --text-main: #1e293b;
  --text-muted: #64748b;
  --border: #e2e8f0;

  /* 间距 */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;

  /* 字体 */
  --font-xs: 12px;
  --font-sm: 14px;
  --font-base: 16px;
  --font-lg: 18px;
  --font-xl: 24px;

  /* 圆角 */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-pill: 9999px;

  /* 阴影 */
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.1);
  --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.1);
}
```

---

## 6. 核心检测算法

### 6.1 检测流程总览

```
输入文本
    ↓
┌─────────────────────────────────────────────────────────────┐
│                    文本预处理阶段                            │
│  1. Unicode 规范化 (NFC)                                    │
│  2. 空白字符标准化                                          │
│  3. 语言检测 (中文/英文/混合)                               │
│  4. 敏感信息脱敏 (可选)                                     │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│                    文本分块阶段                              │
│  段落模式: 按段落边界分割，保持语义完整性                    │
│  句子模式: 按句子边界分割，细粒度分析                        │
│  重叠策略: 相邻块有 10% 重叠，避免边界效应                   │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│                    多信号分析阶段                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ LLM 判断    │  │ 困惑度分析  │  │ 文体特征    │         │
│  │ (主信号)    │  │ (辅助信号)  │  │ (辅助信号)  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│                    结果聚合阶段                              │
│  1. 置信度加权平均                                          │
│  2. 修剪均值 (去除极端值)                                   │
│  3. 对比度锐化                                              │
│  4. 阈值判定                                                │
└─────────────────────────────────────────────────────────────┘
    ↓
输出结果 (概率 + 置信度 + 决策)
```

### 6.2 LLM 判断算法

#### 6.2.1 提示词设计

```rust
// 系统提示词 (中文文本)
const SYSTEM_PROMPT_ZH: &str = r#"
你是一个专业的AI生成文本检测专家。你的任务是分析给定的文本片段，
判断它是由人类撰写还是由AI生成。

评估维度：
1. 词汇多样性：AI文本通常词汇重复率较高
2. 句式结构：AI倾向于使用规整、模板化的句式
3. 逻辑连贯性：AI文本可能存在逻辑跳跃或过度平滑
4. 情感表达：人类文本情感更自然、细腻
5. 创意性：AI文本创意性和个性化程度较低

请以JSON格式返回结果：
{
  "probability": 0.0-1.0,  // AI生成概率
  "confidence": 0.0-1.0,   // 判断置信度
  "reasoning": "..."       // 简要分析理由
}
"#;

// 系统提示词 (英文文本)
const SYSTEM_PROMPT_EN: &str = r#"
You are an expert AI-generated text detector. Analyze the given text
and determine whether it was written by a human or generated by AI.

Evaluation criteria:
1. Vocabulary diversity: AI text tends to have higher repetition
2. Sentence structure: AI prefers regular, templated patterns
3. Logical coherence: AI may have logic jumps or over-smoothing
4. Emotional expression: Human text has more natural emotions
5. Creativity: AI text shows less creativity and personalization

Return result in JSON format:
{
  "probability": 0.0-1.0,  // AI generation probability
  "confidence": 0.0-1.0,   // Judgment confidence
  "reasoning": "..."       // Brief analysis
}
"#;
```

#### 6.2.2 并发调用策略

```rust
/// 并发分析多个文本块
pub async fn analyze_blocks_concurrent(
    blocks: Vec<TextBlock>,
    provider: &str,
    api_key: &str,
) -> Result<Vec<SegmentResponse>> {
    // 限制并发数，避免 API 限流
    let semaphore = Arc::new(Semaphore::new(5));

    let futures: Vec<_> = blocks
        .into_iter()
        .enumerate()
        .map(|(idx, block)| {
            let sem = semaphore.clone();
            let provider = provider.to_string();
            let api_key = api_key.to_string();

            async move {
                // 获取信号量许可
                let _permit = sem.acquire().await?;

                // 调用 LLM API
                let result = call_llm_api(&block.text, &provider, &api_key).await?;

                Ok(SegmentResponse {
                    chunk_id: idx,
                    language: block.language.clone(),
                    offsets: block.offsets.clone(),
                    ai_probability: result.probability,
                    confidence: result.confidence,
                    signals: build_signals(&result),
                    explanations: vec![result.reasoning],
                })
            }
        })
        .collect();

    // 并发执行所有请求
    let results = futures::future::join_all(futures).await;

    // 收集成功结果
    results.into_iter().collect()
}
```

#### 6.2.3 句子长度过滤

```rust
/// 过滤过短的句子，避免噪声
fn filter_short_sentences(blocks: &[TextBlock], min_chars: usize) -> Vec<&TextBlock> {
    blocks
        .iter()
        .filter(|block| {
            let char_count = block.text.chars().count();
            char_count >= min_chars
        })
        .collect()
}

// 默认最小字符数
const MIN_SENTENCE_CHARS_ZH: usize = 10;  // 中文
const MIN_SENTENCE_CHARS_EN: usize = 20;  // 英文
```

### 6.3 文体特征分析 (Stylometry)

#### 6.3.1 特征计算

```rust
/// 计算文体特征
pub fn compute_stylometry(text: &str) -> StylometryFeatures {
    let words = tokenize(text);
    let sentences = split_sentences(text);

    StylometryFeatures {
        // 类型-词符比 (Type-Token Ratio)
        ttr: compute_ttr(&words),

        // 平均句子长度
        avg_sentence_len: compute_avg_sentence_len(&sentences),

        // 功能词比例
        function_word_ratio: compute_function_word_ratio(&words),

        // 重复率
        repeat_ratio: compute_repeat_ratio(&words),

        // 标点符号比例
        punctuation_ratio: compute_punctuation_ratio(text),
    }
}

/// 类型-词符比计算
fn compute_ttr(words: &[String]) -> f64 {
    if words.is_empty() {
        return 0.0;
    }
    let unique: HashSet<_> = words.iter().collect();
    unique.len() as f64 / words.len() as f64
}

/// 平均句子长度
fn compute_avg_sentence_len(sentences: &[String]) -> f64 {
    if sentences.is_empty() {
        return 0.0;
    }
    let total_words: usize = sentences
        .iter()
        .map(|s| tokenize(s).len())
        .sum();
    total_words as f64 / sentences.len() as f64
}
```

#### 6.3.2 AI 特征模式

```rust
/// AI 文本的典型文体特征
const AI_STYLOMETRY_PATTERNS: StylometryPatterns = StylometryPatterns {
    // AI 文本 TTR 通常较低 (词汇重复)
    ttr_threshold: 0.4,

    // AI 文本句子长度较均匀
    sentence_len_variance_threshold: 5.0,

    // AI 文本功能词比例较高
    function_word_ratio_threshold: 0.45,

    // AI 文本重复率较高
    repeat_ratio_threshold: 0.15,
};

/// 基于文体特征计算 AI 概率
fn stylometry_to_probability(features: &StylometryFeatures) -> f64 {
    let mut score = 0.0;
    let mut weight_sum = 0.0;

    // TTR 评分 (权重 0.3)
    if features.ttr < AI_STYLOMETRY_PATTERNS.ttr_threshold {
        score += 0.3 * (1.0 - features.ttr / AI_STYLOMETRY_PATTERNS.ttr_threshold);
    }
    weight_sum += 0.3;

    // 重复率评分 (权重 0.25)
    if features.repeat_ratio > AI_STYLOMETRY_PATTERNS.repeat_ratio_threshold {
        score += 0.25 * (features.repeat_ratio / 0.3).min(1.0);
    }
    weight_sum += 0.25;

    // ... 其他特征评分

    score / weight_sum
}
```

### 6.4 结果聚合算法

#### 6.4.1 软阈值系统

```rust
/// 检测阈值配置
pub struct DetectionThresholds {
    pub low: f64,       // 0.65 - 低风险阈值
    pub medium: f64,    // 0.75 - 中风险阈值
    pub high: f64,      // 0.85 - 高风险阈值
    pub very_high: f64, // 0.90 - 极高风险阈值
}

impl Default for DetectionThresholds {
    fn default() -> Self {
        Self {
            low: 0.65,
            medium: 0.75,
            high: 0.85,
            very_high: 0.90,
        }
    }
}

/// 根据概率和阈值生成决策
fn derive_decision(probability: f64, thresholds: &DetectionThresholds) -> Decision {
    if probability < thresholds.low {
        Decision::Pass  // 通过，可能是人类撰写
    } else if probability < thresholds.medium {
        Decision::Review  // 需要人工审核
    } else {
        Decision::Flag  // 标记为可能的 AI 生成
    }
}
```

#### 6.4.2 置信度加权聚合

```rust
/// 聚合多个段落的检测结果
pub fn aggregate_segments(segments: &[SegmentResponse]) -> AggregationResponse {
    if segments.is_empty() {
        return AggregationResponse::default();
    }

    // 1. 计算置信度加权平均
    let (weighted_sum, weight_total) = segments.iter().fold(
        (0.0, 0.0),
        |(sum, total), seg| {
            let weight = seg.confidence;
            (sum + seg.ai_probability * weight, total + weight)
        },
    );

    let weighted_avg = if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        0.5
    };

    // 2. 计算修剪均值 (去除最高和最低 10%)
    let trimmed_mean = compute_trimmed_mean(
        &segments.iter().map(|s| s.ai_probability).collect::<Vec<_>>(),
        0.1,
    );

    // 3. 综合两种方法
    let overall_probability = weighted_avg * 0.7 + trimmed_mean * 0.3;

    // 4. 计算整体置信度
    let overall_confidence = compute_overall_confidence(segments);

    // 5. 应用对比度锐化
    let sharpened = apply_contrast_sharpening(overall_probability);

    AggregationResponse {
        overall_probability: sharpened,
        overall_confidence,
        method: "weighted_trimmed_mean".to_string(),
        thresholds: DetectionThresholds::default(),
        decision: derive_decision(sharpened, &DetectionThresholds::default()),
        ..Default::default()
    }
}
```

#### 6.4.3 对比度锐化

```rust
/// 对比度锐化：增强高/低概率的区分度
/// 使用 logit 空间变换
fn apply_contrast_sharpening(probability: f64) -> f64 {
    // 避免极端值
    let p = probability.clamp(0.01, 0.99);

    // 转换到 logit 空间
    let logit = (p / (1.0 - p)).ln();

    // 应用锐化因子 (1.2 = 轻度锐化)
    let sharpening_factor = 1.2;
    let sharpened_logit = logit * sharpening_factor;

    // 转换回概率空间
    let sharpened = 1.0 / (1.0 + (-sharpened_logit).exp());

    // 限制在有效范围内
    sharpened.clamp(0.0, 1.0)
}
```

### 6.5 双模式检测

#### 6.5.1 段落模式 vs 句子模式

```rust
/// 双模式检测结果
pub struct DualModeResult {
    /// 段落级检测结果
    pub paragraph: ModeDetectionResult,
    /// 句子级检测结果
    pub sentence: ModeDetectionResult,
    /// 两种模式的比较分析
    pub comparison: ComparisonResult,
    /// 融合后的最终结果
    pub fused: AggregationResponse,
}

/// 执行双模式检测
pub async fn detect_dual_mode(
    text: &str,
    provider: &str,
    api_key: &str,
) -> Result<DualModeResult> {
    // 并行执行两种模式的检测
    let (paragraph_result, sentence_result) = tokio::join!(
        detect_paragraph_mode(text, provider, api_key),
        detect_sentence_mode(text, provider, api_key),
    );

    let paragraph = paragraph_result?;
    let sentence = sentence_result?;

    // 比较两种模式的结果
    let comparison = compare_modes(&paragraph, &sentence);

    // 融合结果 (段落权重 0.6，句子权重 0.4)
    let fused = fuse_results(&paragraph, &sentence, 0.6, 0.4);

    Ok(DualModeResult {
        paragraph,
        sentence,
        comparison,
        fused,
    })
}
```

#### 6.5.2 结果融合策略

```rust
/// 融合段落和句子检测结果
fn fuse_results(
    paragraph: &ModeDetectionResult,
    sentence: &ModeDetectionResult,
    para_weight: f64,
    sent_weight: f64,
) -> AggregationResponse {
    // 加权融合概率
    let fused_probability =
        paragraph.aggregation.overall_probability * para_weight +
        sentence.aggregation.overall_probability * sent_weight;

    // 融合置信度 (取较高者的加权平均)
    let fused_confidence =
        paragraph.aggregation.overall_confidence.max(
            sentence.aggregation.overall_confidence
        ) * 0.8 +
        paragraph.aggregation.overall_confidence.min(
            sentence.aggregation.overall_confidence
        ) * 0.2;

    AggregationResponse {
        overall_probability: fused_probability,
        overall_confidence: fused_confidence,
        method: "dual_mode_fusion".to_string(),
        decision: derive_decision(fused_probability, &DetectionThresholds::default()),
        ..Default::default()
    }
}
```

#### 6.5.3 差异区域检测

```rust
/// 比较两种模式的结果，找出差异区域
fn compare_modes(
    paragraph: &ModeDetectionResult,
    sentence: &ModeDetectionResult,
) -> ComparisonResult {
    // 计算整体概率差异
    let probability_diff = (
        paragraph.aggregation.overall_probability -
        sentence.aggregation.overall_probability
    ).abs();

    // 计算一致性分数
    let consistency_score = 1.0 - probability_diff;

    // 找出差异显著的区域
    let divergent_regions = find_divergent_regions(
        &paragraph.segments,
        &sentence.segments,
        0.2,  // 差异阈值
    );

    ComparisonResult {
        probability_diff,
        consistency_score,
        divergent_regions,
    }
}
```

---

## 7. 数据模型与类型系统

### 7.1 Rust 后端数据模型

#### 7.1.1 检测请求/响应模型

```rust
// src-tauri/src/models/mod.rs

/// 检测请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectRequest {
    /// 待检测文本
    pub text: String,
    /// 是否使用困惑度分析
    pub use_perplexity: bool,
    /// 是否使用文体特征分析
    pub use_stylometry: bool,
    /// 敏感度级别: "low" | "medium" | "high"
    pub sensitivity: String,
    /// AI 提供商: "glm" | "deepseek" | null
    pub provider: Option<String>,
    /// 是否启用双模式检测
    pub dual_mode: bool,
}

/// 检测响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectResponse {
    /// 聚合结果
    pub aggregation: AggregationResponse,
    /// 各段落检测结果
    pub segments: Vec<SegmentResponse>,
    /// 预处理摘要
    pub preprocess_summary: PreprocessSummary,
    /// 成本信息
    pub cost: CostInfo,
    /// API 版本
    pub version: String,
    /// 请求 ID
    pub request_id: String,
    /// 双模式检测结果 (可选)
    pub dual_detection: Option<DualDetectionResult>,
}
```

#### 7.1.2 段落检测结果

```rust
/// 单个段落的检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentResponse {
    /// 段落 ID (0-based)
    pub chunk_id: usize,
    /// 检测到的语言
    pub language: String,
    /// 文本偏移量 (UTF-8 字节)
    pub offsets: Offsets,
    /// AI 生成概率 (0.0-1.0)
    pub ai_probability: f64,
    /// 判断置信度 (0.0-1.0)
    pub confidence: f64,
    /// 多维度信号
    pub signals: SegmentSignals,
    /// 分析说明
    pub explanations: Vec<String>,
}

/// 文本偏移量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offsets {
    /// 起始字节位置 (0-based, inclusive)
    pub start: usize,
    /// 结束字节位置 (0-based, exclusive)
    pub end: usize,
}

/// 多维度检测信号
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSignals {
    /// LLM 判断信号
    pub llm_judgment: LlmJudgment,
    /// 困惑度信号
    pub perplexity: PerplexitySignal,
    /// 文体特征信号
    pub stylometry: StylometrySignal,
}

/// LLM 判断结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmJudgment {
    /// AI 概率
    pub prob: Option<f64>,
    /// 使用的模型列表
    pub models: Vec<String>,
}

/// 困惑度信号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerplexitySignal {
    /// 困惑度值
    pub ppl: Option<f64>,
    /// Z-score 标准化值
    pub z: Option<f64>,
}

/// 文体特征信号
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StylometrySignal {
    /// 类型-词符比
    pub ttr: f64,
    /// 平均句子长度
    pub avg_sentence_len: f64,
    /// 功能词比例
    pub function_word_ratio: Option<f64>,
    /// 重复率
    pub repeat_ratio: Option<f64>,
    /// 标点符号比例
    pub punctuation_ratio: Option<f64>,
}
```

#### 7.1.3 聚合结果模型

```rust
/// 聚合检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationResponse {
    /// 整体 AI 概率
    pub overall_probability: f64,
    /// 整体置信度
    pub overall_confidence: f64,
    /// 聚合方法
    pub method: String,
    /// 阈值配置
    pub thresholds: Thresholds,
    /// 评分标准版本
    pub rubric_version: String,
    /// 决策结果: "pass" | "review" | "flag"
    pub decision: String,
    /// 缓冲边距
    pub buffer_margin: f64,
    /// 文体特征概率
    pub stylometry_probability: Option<f64>,
    /// 质量分数 (归一化)
    pub quality_score_normalized: Option<f64>,
}

/// 检测阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    pub low: f64,       // 0.65
    pub medium: f64,    // 0.75
    pub high: f64,      // 0.85
    pub very_high: f64, // 0.90
}
```

#### 7.1.4 双模式检测模型

```rust
/// 双模式检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DualDetectionResult {
    /// 段落模式结果
    pub paragraph: ModeDetectionResult,
    /// 句子模式结果
    pub sentence: ModeDetectionResult,
    /// 比较分析
    pub comparison: ComparisonResult,
    /// 融合聚合结果
    pub fused_aggregation: Option<AggregationResponse>,
}

/// 单模式检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeDetectionResult {
    /// 聚合结果
    pub aggregation: AggregationResponse,
    /// 段落结果列表
    pub segments: Vec<SegmentResponse>,
    /// 段落数量
    pub segment_count: usize,
}

/// 模式比较结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonResult {
    /// 概率差异
    pub probability_diff: f64,
    /// 一致性分数
    pub consistency_score: f64,
    /// 差异区域列表
    pub divergent_regions: Vec<DivergentRegion>,
}

/// 差异区域
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivergentRegion {
    /// 段落模式段落 ID
    pub paragraph_segment_id: usize,
    /// 句子模式段落 ID
    pub sentence_segment_id: usize,
    /// 概率差异
    pub probability_diff: f64,
    /// 段落模式概率
    pub paragraph_prob: f64,
    /// 句子模式概率
    pub sentence_prob: f64,
    /// 文本预览
    pub text_preview: String,
}
```

#### 7.1.5 配置模型

```rust
/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// 默认 AI 提供商
    pub default_provider: String,
    /// 敏感度级别
    pub sensitivity: String,
    /// 是否启用困惑度分析
    pub use_perplexity: bool,
    /// 是否启用文体特征分析
    pub use_stylometry: bool,
    /// 是否默认启用双模式
    pub dual_mode: bool,
}

/// AI 提供商信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    /// 提供商标识
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 是否已配置 API Key
    pub has_key: bool,
}
```

### 7.2 TypeScript 前端类型

#### 7.2.1 核心类型定义

```typescript
// src/types/index.ts

// 提供商类型
export interface ProviderInfo {
  name: string;
  display_name: string;
  has_key: boolean;
}

export interface ProviderOption {
  value: string;
  label: string;
}

// 敏感度类型
export interface SensitivityOption {
  value: string;
  label: string;
}

export const SENSITIVITY_OPTIONS: SensitivityOption[] = [
  { value: "low", label: "低敏感" },
  { value: "medium", label: "中敏感" },
  { value: "high", label: "高敏感" },
];
```

#### 7.2.2 检测结果类型

```typescript
// 段落信号
export interface SegmentSignals {
  llm_judgment: {
    prob: number | null;
    models: string[];
  };
  perplexity: {
    ppl: number | null;
    z: number | null;
  };
  stylometry: {
    ttr: number;
    avg_sentence_len: number;
    function_word_ratio: number | null;
    repeat_ratio: number | null;
    punctuation_ratio: number | null;
  };
}

// 段落响应
export interface SegmentResponse {
  chunkId: number;
  language: string;
  offsets: {
    /** UTF-8 byte offsets (0-based, end-exclusive) */
    start: number;
    end: number;
  };
  aiProbability: number;
  confidence: number;
  signals: SegmentSignals;
  explanations: string[];
}

// 聚合响应
export interface AggregationResponse {
  overallProbability: number;
  overallConfidence: number;
  method: string;
  thresholds: {
    low: number;
    medium: number;
    high: number;
    veryHigh: number;
  };
  rubricVersion: string;
  decision: string;
  bufferMargin: number;
  stylometryProbability: number | null;
  qualityScoreNormalized: number | null;
}
```

#### 7.2.3 双模式检测类型

```typescript
// 单模式结果
export interface ModeDetectionResult {
  aggregation: AggregationResponse;
  segments: SegmentResponse[];
  segmentCount: number;
}

// 比较结果
export interface ComparisonResult {
  probabilityDiff: number;
  consistencyScore: number;
  divergentRegions: Array<{
    paragraphSegmentId: number;
    sentenceSegmentId: number;
    probabilityDiff: number;
    paragraphProb: number;
    sentenceProb: number;
    textPreview: string;
  }>;
}

// 双模式检测结果
export interface DualDetectionResult {
  paragraph: ModeDetectionResult;
  sentence: ModeDetectionResult;
  comparison: ComparisonResult;
  /** Fused aggregation (weight: paragraph 0.6 + sentence 0.4) */
  fusedAggregation?: AggregationResponse;
}

// 完整检测响应
export interface DetectResponse {
  aggregation: AggregationResponse;
  segments: SegmentResponse[];
  preprocessSummary: {
    language: string;
    chunks: number;
    redacted: number;
  };
  cost: {
    tokens: number;
    latencyMs: number;
  };
  version: string;
  requestId: string;
  dualDetection: DualDetectionResult | null;
}
```

#### 7.2.4 UI 状态类型

```typescript
// 决策类型
export type DecisionType = 'pass' | 'review' | 'flag';

// UI 状态
export interface UIState {
  isLoading: boolean;
  loadingText: string;
  sensitivityOpen: boolean;
  providerOpen: boolean;
  settingsOpen: boolean;
}

// 检测请求
export interface DetectTextRequest {
  text: string;
  usePerplexity: boolean;
  useStylometry: boolean;
  sensitivity: string;
  provider: string | null;
  dualMode: boolean;
}
```

### 7.3 类型转换与序列化

#### 7.3.1 Rust 到 TypeScript 的映射

| Rust 类型 | TypeScript 类型 | 说明 |
|-----------|-----------------|------|
| `String` | `string` | 字符串 |
| `usize` | `number` | 无符号整数 |
| `f64` | `number` | 浮点数 |
| `bool` | `boolean` | 布尔值 |
| `Option<T>` | `T \| null` | 可选值 |
| `Vec<T>` | `T[]` | 数组 |
| `HashMap<K, V>` | `Record<K, V>` | 映射 |

#### 7.3.2 Serde 序列化配置

```rust
// 使用 camelCase 命名约定
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub field_name: String,  // 序列化为 "fieldName"
}

// 跳过 None 值
#[derive(Serialize, Deserialize)]
pub struct OptionalFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,
}

// 默认值
#[derive(Serialize, Deserialize)]
pub struct WithDefaults {
    #[serde(default)]
    pub count: usize,  // 默认为 0
}
```

---

## 8. API 接口文档

### 8.1 Tauri 命令接口

#### 8.1.1 检测命令

##### `detect_text` - 单模式文本检测

```typescript
// 调用方式
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<DetectResponse>('detect_text', {
  request: {
    text: "待检测的文本内容...",
    usePerplexity: false,
    useStylometry: true,
    sensitivity: "medium",
    provider: "deepseek",
    dualMode: false
  }
});
```

```rust
// Rust 实现
#[tauri::command]
pub async fn detect_text(request: DetectRequest) -> Result<DetectResponse, String> {
    // 1. 验证请求参数
    validate_request(&request)?;

    // 2. 获取 API Key
    let api_key = get_api_key_for_provider(&request.provider)?;

    // 3. 预处理文本
    let processed = preprocess_text(&request.text)?;

    // 4. 执行检测
    let segments = analyze_segments(&processed, &request, &api_key).await?;

    // 5. 聚合结果
    let aggregation = aggregate_segments(&segments);

    // 6. 构建响应
    Ok(DetectResponse {
        aggregation,
        segments,
        preprocess_summary: processed.summary,
        cost: calculate_cost(&segments),
        version: env!("CARGO_PKG_VERSION").to_string(),
        request_id: generate_request_id(),
        dual_detection: None,
    })
}
```

##### `detect_dual_mode` - 双模式文本检测

```typescript
// 调用方式
const result = await invoke<DetectResponse>('detect_dual_mode', {
  request: {
    text: "待检测的文本内容...",
    usePerplexity: false,
    useStylometry: true,
    sensitivity: "medium",
    provider: "deepseek",
    dualMode: true
  }
});
```

```rust
// Rust 实现
#[tauri::command]
pub async fn detect_dual_mode(request: DetectRequest) -> Result<DetectResponse, String> {
    // 1. 并行执行段落和句子模式检测
    let (paragraph_result, sentence_result) = tokio::join!(
        detect_paragraph_mode(&request),
        detect_sentence_mode(&request),
    );

    // 2. 比较两种模式结果
    let comparison = compare_modes(&paragraph_result?, &sentence_result?);

    // 3. 融合结果
    let fused = fuse_results(&paragraph_result?, &sentence_result?);

    // 4. 构建双模式响应
    Ok(DetectResponse {
        aggregation: fused.clone(),
        segments: paragraph_result?.segments,
        dual_detection: Some(DualDetectionResult {
            paragraph: paragraph_result?,
            sentence: sentence_result?,
            comparison,
            fused_aggregation: Some(fused),
        }),
        ..Default::default()
    })
}
```

##### `preprocess_file` - 文件预处理

```typescript
// 调用方式
const text = await invoke<string>('preprocess_file', {
  filePath: "C:/Documents/report.docx"
});
```

```rust
// Rust 实现
#[tauri::command]
pub async fn preprocess_file(file_path: String) -> Result<String, String> {
    let path = Path::new(&file_path);
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "docx" => extract_docx_text(&file_path),
        "txt" => std::fs::read_to_string(&file_path)
            .map_err(|e| e.to_string()),
        "pdf" => extract_pdf_text(&file_path),
        _ => Err(format!("Unsupported file type: {}", extension)),
    }
}
```

#### 8.1.2 配置命令

##### `get_config` - 获取配置

```typescript
const config = await invoke<AppConfig>('get_config');
```

```rust
#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    config_store::load_config()
}
```

##### `save_config` - 保存配置

```typescript
await invoke('save_config', {
  config: {
    defaultProvider: "deepseek",
    sensitivity: "medium",
    usePerplexity: false,
    useStylometry: true,
    dualMode: true
  }
});
```

```rust
#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config_store::save_config(&config)
}
```

##### `get_providers` - 获取提供商列表

```typescript
const providers = await invoke<ProviderInfo[]>('get_providers');
// 返回: [
//   { name: "glm", display_name: "智谱 GLM", has_key: true },
//   { name: "deepseek", display_name: "DeepSeek", has_key: false }
// ]
```

```rust
#[tauri::command]
pub fn get_providers() -> Result<Vec<ProviderInfo>, String> {
    Ok(vec![
        ProviderInfo {
            name: "glm".to_string(),
            display_name: "智谱 GLM".to_string(),
            has_key: has_api_key("glm"),
        },
        ProviderInfo {
            name: "deepseek".to_string(),
            display_name: "DeepSeek".to_string(),
            has_key: has_api_key("deepseek"),
        },
    ])
}
```

#### 8.1.3 API Key 管理命令

##### `store_api_key` - 存储 API Key

```typescript
await invoke('store_api_key', {
  provider: "deepseek",
  apiKey: "sk-xxxxxxxxxxxxxxxx"
});
```

```rust
#[tauri::command]
pub fn store_api_key(provider: String, api_key: String) -> Result<(), String> {
    // 使用系统凭据存储
    let entry = keyring::Entry::new("cheekAI", &provider)
        .map_err(|e| e.to_string())?;
    entry.set_password(&api_key)
        .map_err(|e| e.to_string())
}
```

##### `get_api_key` - 获取 API Key

```typescript
const apiKey = await invoke<string | null>('get_api_key', {
  provider: "deepseek"
});
```

```rust
#[tauri::command]
pub fn get_api_key(provider: String) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("cheekAI", &provider)
        .map_err(|e| e.to_string())?;

    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
```

##### `delete_api_key` - 删除 API Key

```typescript
await invoke('delete_api_key', { provider: "deepseek" });
```

```rust
#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    let entry = keyring::Entry::new("cheekAI", &provider)
        .map_err(|e| e.to_string())?;
    entry.delete_credential()
        .map_err(|e| e.to_string())
}
```

#### 8.1.4 诊断命令

##### `diagnose_api_config` - API 配置诊断

```typescript
const diagnosis = await invoke<DiagnosisResult>('diagnose_api_config', {
  provider: "deepseek"
});
```

##### `test_api_connection` - 测试 API 连接

```typescript
const testResult = await invoke<ConnectionTestResult>('test_api_connection', {
  provider: "deepseek"
});
```

##### `get_provider_url` / `set_provider_url` - 自定义 API URL

```typescript
// 获取当前 URL
const url = await invoke<string>('get_provider_url', { provider: "deepseek" });

// 设置自定义 URL
await invoke('set_provider_url', {
  provider: "deepseek",
  url: "https://custom-api.example.com/v1"
});
```

### 8.2 外部 AI API 集成

#### 8.2.1 DeepSeek API

```rust
// API 端点
const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/v1/chat/completions";

// 请求结构
#[derive(Serialize)]
struct DeepSeekRequest {
    model: String,           // "deepseek-chat"
    messages: Vec<Message>,
    temperature: f64,        // 0.0-2.0
    max_tokens: u32,         // 最大输出 token
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct Message {
    role: String,    // "system" | "user" | "assistant"
    content: String,
}

// 响应结构
#[derive(Deserialize)]
struct DeepSeekResponse {
    id: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
    finish_reason: String,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
```

#### 8.2.2 智谱 GLM API

```rust
// API 端点
const GLM_API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

// 请求结构 (与 OpenAI 兼容)
#[derive(Serialize)]
struct GlmRequest {
    model: String,           // "glm-4-flash"
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: u32,
}

// 认证方式
// Header: Authorization: Bearer {api_key}
```

#### 8.2.3 API 调用封装

```rust
/// 统一的 AI 提供商客户端
pub struct AiProviderClient {
    http_client: reqwest::Client,
    provider: String,
    api_key: String,
    base_url: String,
}

impl AiProviderClient {
    /// 创建新客户端
    pub fn new(provider: &str, api_key: &str) -> Self {
        let base_url = match provider {
            "deepseek" => DEEPSEEK_API_URL.to_string(),
            "glm" => GLM_API_URL.to_string(),
            _ => panic!("Unknown provider: {}", provider),
        };

        Self {
            http_client: reqwest::Client::new(),
            provider: provider.to_string(),
            api_key: api_key.to_string(),
            base_url,
        }
    }

    /// 发送检测请求
    pub async fn detect(&self, text: &str) -> Result<DetectionResult> {
        let request = self.build_request(text);

        let response = self.http_client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let result: ApiResponse = response.json().await?;
        self.parse_response(result)
    }
}
```

### 8.3 错误处理

#### 8.3.1 错误类型

```rust
/// 应用错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("File processing error: {0}")]
    FileError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

// 转换为 Tauri 命令错误
impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}
```

#### 8.3.2 前端错误处理

```typescript
// composables/useDetection.ts

async function runDetection() {
  try {
    isLoading.value = true;
    loadingText.value = "正在分析文本...";

    const result = await invoke<DetectResponse>('detect_text', { request });
    detectionResult.value = result;

  } catch (error) {
    // 解析错误类型
    const errorMessage = error instanceof Error
      ? error.message
      : String(error);

    // 显示用户友好的错误信息
    if (errorMessage.includes("API")) {
      showError("API 调用失败，请检查网络连接和 API Key");
    } else if (errorMessage.includes("timeout")) {
      showError("请求超时，请稍后重试");
    } else {
      showError(`检测失败: ${errorMessage}`);
    }

  } finally {
    isLoading.value = false;
    loadingText.value = "";
  }
}
```

---

## 9. 配置与环境变量

### 9.1 应用配置

#### 9.1.1 配置文件位置

```
Windows: %APPDATA%\cheekAI\config.json
macOS:   ~/Library/Application Support/cheekAI/config.json
Linux:   ~/.config/cheekAI/config.json
```

#### 9.1.2 配置文件结构

```json
{
  "defaultProvider": "deepseek",
  "sensitivity": "medium",
  "usePerplexity": false,
  "useStylometry": true,
  "dualMode": true,
  "customUrls": {
    "deepseek": null,
    "glm": null
  }
}
```

#### 9.1.3 配置项说明

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `defaultProvider` | string | `"deepseek"` | 默认 AI 提供商 |
| `sensitivity` | string | `"medium"` | 检测敏感度: low/medium/high |
| `usePerplexity` | boolean | `false` | 是否启用困惑度分析 |
| `useStylometry` | boolean | `true` | 是否启用文体特征分析 |
| `dualMode` | boolean | `true` | 是否默认启用双模式检测 |
| `customUrls` | object | `{}` | 自定义 API URL |

### 9.2 环境变量

#### 9.2.1 API Key 环境变量

```bash
# DeepSeek API Key
DEEPSEEK_API_KEY=sk-xxxxxxxxxxxxxxxx
# 或
CHEEKAI_DEEPSEEK_API_KEY=sk-xxxxxxxxxxxxxxxx

# 智谱 GLM API Key
GLM_API_KEY=xxxxxxxxxxxxxxxx
# 或
CHEEKAI_GLM_API_KEY=xxxxxxxxxxxxxxxx
```

#### 9.2.2 日志配置

```bash
# 日志级别 (trace, debug, info, warn, error)
RUST_LOG=info

# 详细日志
RUST_LOG=cheek_ai=debug,reqwest=warn

# 完整调试
RUST_LOG=trace
```

#### 9.2.3 开发环境变量

```bash
# Tauri 开发模式
TAURI_DEBUG=1

# 禁用沙箱 (仅开发)
WEBKIT_DISABLE_COMPOSITING_MODE=1
```

### 9.3 API Key 存储

#### 9.3.1 系统凭据存储

```rust
// Windows: Windows Credential Manager
// macOS: Keychain
// Linux: Secret Service (libsecret)

use keyring::Entry;

// 存储 API Key
fn store_api_key(provider: &str, api_key: &str) -> Result<()> {
    let entry = Entry::new("cheekAI", provider)?;
    entry.set_password(api_key)?;
    Ok(())
}

// 读取 API Key
fn get_api_key(provider: &str) -> Result<Option<String>> {
    let entry = Entry::new("cheekAI", provider)?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```

#### 9.3.2 API Key 优先级

```rust
/// API Key 获取优先级:
/// 1. 系统凭据存储 (keyring)
/// 2. 环境变量 CHEEKAI_{PROVIDER}_API_KEY
/// 3. 环境变量 {PROVIDER}_API_KEY
fn resolve_api_key(provider: &str) -> Option<String> {
    // 1. 尝试从 keyring 获取
    if let Ok(Some(key)) = get_api_key_from_keyring(provider) {
        return Some(key);
    }

    // 2. 尝试 CHEEKAI_ 前缀环境变量
    let env_key = format!("CHEEKAI_{}_API_KEY", provider.to_uppercase());
    if let Ok(key) = std::env::var(&env_key) {
        return Some(key);
    }

    // 3. 尝试标准环境变量
    let env_key = format!("{}_API_KEY", provider.to_uppercase());
    if let Ok(key) = std::env::var(&env_key) {
        return Some(key);
    }

    None
}
```

### 9.4 日志系统

#### 9.4.1 日志文件位置

```
开发模式: ./logs/cheekAI_YYYYMMDD.log
生产模式:
  Windows: %LOCALAPPDATA%\cheekAI\logs\
  macOS:   ~/Library/Logs/cheekAI/
  Linux:   ~/.local/share/cheekAI/logs/
```

#### 9.4.2 日志格式

```
2024-01-15T10:30:45.123Z INFO  cheek_ai::api::detect > Starting detection request
2024-01-15T10:30:45.456Z DEBUG cheek_ai::services::providers > API call to deepseek
2024-01-15T10:30:46.789Z INFO  cheek_ai::api::detect > Detection completed in 1666ms
```

#### 9.4.3 日志配置代码

```rust
// src-tauri/src/lib.rs

fn init_logging() {
    let logs_dir = get_logs_dir();
    fs::create_dir_all(&logs_dir).ok();

    let file_appender = RollingFileAppender::builder()
        .filename_prefix("cheekAI")
        .filename_suffix(".log")
        .max_log_files(30)  // 保留最近 30 个日志文件
        .build(&logs_dir)
        .expect("Failed to create log appender");

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(file_appender))
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();
}
```

---

## 10. 开发与构建指南

### 10.1 开发环境搭建

#### 10.1.1 系统要求

| 组件 | 最低版本 | 推荐版本 |
|------|----------|----------|
| Node.js | 18.x | 20.x LTS |
| Rust | 1.70 | 1.75+ |
| npm | 9.x | 10.x |
| Cargo | 1.70 | 1.75+ |

#### 10.1.2 Windows 额外依赖

```powershell
# 安装 Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# 安装 WebView2 (Windows 10/11 通常已预装)
winget install Microsoft.EdgeWebView2Runtime
```

#### 10.1.3 macOS 额外依赖

```bash
# 安装 Xcode Command Line Tools
xcode-select --install
```

#### 10.1.4 Linux 额外依赖

```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel \
    openssl-devel \
    curl \
    wget \
    file \
    libappindicator-gtk3-devel \
    librsvg2-devel
```

### 10.2 项目初始化

```bash
# 克隆项目
git clone https://github.com/your-org/cheekAI.git
cd cheekAI

# 安装前端依赖
npm install

# 安装 Rust 依赖 (自动)
cd src-tauri
cargo build
cd ..
```

### 10.3 开发命令

#### 10.3.1 启动开发服务器

```bash
# 启动完整开发环境 (前端 + 后端热重载)
npm run tauri dev

# 仅启动前端开发服务器
npm run dev

# 仅检查 Rust 代码
cd src-tauri && cargo check
```

#### 10.3.2 代码检查

```bash
# TypeScript 类型检查
npm run type-check

# ESLint 检查
npm run lint

# Rust 格式检查
cd src-tauri && cargo fmt --check

# Rust clippy 检查
cd src-tauri && cargo clippy
```

#### 10.3.3 运行测试

```bash
# Rust 单元测试
cd src-tauri && cargo test

# Rust 测试 (显示输出)
cd src-tauri && cargo test -- --nocapture
```

### 10.4 构建发布版本

#### 10.4.1 构建命令

```bash
# 构建生产版本
npm run tauri build

# 构建调试版本
npm run tauri build -- --debug

# 指定目标平台 (交叉编译)
npm run tauri build -- --target x86_64-pc-windows-msvc
npm run tauri build -- --target aarch64-apple-darwin
npm run tauri build -- --target x86_64-unknown-linux-gnu
```

#### 10.4.2 构建产物位置

```
Windows:
  src-tauri/target/release/cheekAI.exe
  src-tauri/target/release/bundle/msi/cheekAI_x.x.x_x64.msi
  src-tauri/target/release/bundle/nsis/cheekAI_x.x.x_x64-setup.exe

macOS:
  src-tauri/target/release/bundle/dmg/cheekAI_x.x.x_x64.dmg
  src-tauri/target/release/bundle/macos/cheekAI.app

Linux:
  src-tauri/target/release/bundle/deb/cheekAI_x.x.x_amd64.deb
  src-tauri/target/release/bundle/appimage/cheekAI_x.x.x_amd64.AppImage
```

### 10.5 Tauri 配置

#### 10.5.1 tauri.conf.json 关键配置

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "CheekAI",
  "version": "0.1.0",
  "identifier": "com.cheekAI.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "CheekAI - AI文本检测",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "decorations": false,
        "transparent": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

#### 10.5.2 Cargo.toml 关键配置

```toml
[package]
name = "cheek-ai"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
keyring = "3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
chrono = "0.4"
uuid = { version = "1", features = ["v4"] }
zip = "2"
quick-xml = "0.36"
dirs = "5"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 10.6 项目脚本

#### 10.6.1 package.json 脚本

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc -b && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "type-check": "vue-tsc --noEmit",
    "lint": "eslint . --ext .vue,.js,.jsx,.cjs,.mjs,.ts,.tsx --fix"
  }
}
```

#### 10.6.2 开发辅助脚本

```powershell
# scripts/dev-onekey.ps1 - Windows 一键启动
Write-Host "Starting CheekAI Development Environment..."
npm run tauri dev
```

### 10.7 调试技巧

#### 10.7.1 Rust 后端调试

```rust
// 使用 tracing 宏记录日志
use tracing::{info, debug, warn, error};

info!("Processing request: {:?}", request);
debug!("Intermediate result: {:?}", result);
warn!("Potential issue: {}", message);
error!("Error occurred: {:?}", error);

// 使用 dbg! 宏快速调试
let result = dbg!(some_function());
```

#### 10.7.2 前端调试

```typescript
// 使用 Vue DevTools
// Chrome: 安装 Vue.js devtools 扩展

// 控制台调试
console.log('Detection result:', result);
console.table(segments);

// 使用 debugger 断点
debugger;
```

#### 10.7.3 Tauri DevTools

```bash
# 开发模式自动启用 DevTools
npm run tauri dev

# 生产版本启用 DevTools (需要在 Cargo.toml 中配置)
[dependencies]
tauri = { version = "2", features = ["devtools"] }
```

### 10.8 常见问题排查

#### 10.8.1 构建失败

```bash
# 清理并重新构建
cd src-tauri && cargo clean
npm run tauri build

# 更新依赖
npm update
cd src-tauri && cargo update
```

#### 10.8.2 API 调用失败

```bash
# 检查 API Key 是否正确配置
# 1. 检查环境变量
echo $DEEPSEEK_API_KEY

# 2. 检查系统凭据存储
# Windows: 控制面板 > 凭据管理器
# macOS: 钥匙串访问
```

#### 10.8.3 窗口不显示

```bash
# 检查 WebView2 是否安装 (Windows)
# 检查 webkit2gtk 是否安装 (Linux)

# 尝试禁用硬件加速
WEBKIT_DISABLE_COMPOSITING_MODE=1 npm run tauri dev
```

---

## 附录

### A. 术语表

| 术语 | 英文 | 说明 |
|------|------|------|
| 困惑度 | Perplexity | 语言模型对文本的困惑程度，AI 生成文本通常困惑度较低 |
| 文体特征 | Stylometry | 通过统计特征分析文本风格 |
| TTR | Type-Token Ratio | 类型-词符比，衡量词汇多样性 |
| 双模式检测 | Dual Mode Detection | 同时使用段落和句子级别进行检测 |
| 对比度锐化 | Contrast Sharpening | 增强高/低概率的区分度 |
| Logit 空间 | Logit Space | 概率的对数几率变换空间 |

### B. 参考资料

- [Tauri 官方文档](https://tauri.app/v2/guides/)
- [Vue 3 文档](https://vuejs.org/guide/)
- [Rust 官方文档](https://doc.rust-lang.org/book/)
- [DeepSeek API 文档](https://platform.deepseek.com/api-docs/)
- [智谱 GLM API 文档](https://open.bigmodel.cn/dev/api)

### C. 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.1.0 | 2024-01 | 初始版本，基础检测功能 |
| 0.2.0 | 2024-02 | 添加双模式检测 |
| 0.3.0 | 2024-03 | 优化算法，添加文体特征分析 |

---

*文档生成时间: 2024-01-15*
*文档版本: 1.0.0*
