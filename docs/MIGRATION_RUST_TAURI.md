# CheekAI 技术栈迁移：Python + Electron → Rust + Tauri

## 迁移概述

| 原技术栈 | 新技术栈 |
|---------|---------|
| Python 3.8+ (FastAPI) | Rust (Axum/Actix-web) |
| Electron 28 | Tauri 2.x |
| JavaScript (Renderer) | TypeScript + 前端框架 (Vue/React/Svelte) |
| uvicorn | 内置 Rust HTTP server |
| keytar | tauri-plugin-store / keyring-rs |

## 新项目结构

```
cheekAI/
├── src-tauri/              # Rust 后端 + Tauri 集成
│   ├── src/
│   │   ├── main.rs         # Tauri 入口
│   │   ├── lib.rs          # 库入口
│   │   ├── api/            # API 命令 (Tauri commands)
│   │   │   ├── mod.rs
│   │   │   ├── detect.rs   # 检测逻辑
│   │   │   └── config.rs   # 配置管理
│   │   ├── services/       # 业务逻辑
│   │   │   ├── mod.rs
│   │   │   ├── detection/      # 检测核心模块 (已拆分)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── segment_builder.rs  # 分段构建
│   │   │   │   ├── aggregation.rs      # 结果聚合
│   │   │   │   ├── comparison.rs       # 双模式对比
│   │   │   │   ├── dual_mode.rs        # 双模式检测
│   │   │   │   └── llm_analyzer.rs     # LLM 分析
│   │   │   ├── providers.rs        # AI 提供商 (GLM/Deepseek)
│   │   │   ├── text_processor.rs   # 文本预处理
│   │   │   ├── config_store.rs     # 配置存储
│   │   │   └── sentence_segmenter.rs # 分句器
│   │   └── models/         # 数据结构
│   │       └── mod.rs      # 请求/响应结构
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                    # 前端 (Vue 3 + TypeScript)
│   ├── App.vue             # 主应用组件
│   ├── main.ts             # 前端入口
│   ├── components/         # UI 组件
│   │   ├── TitleBar.vue
│   │   ├── ControlPanel.vue
│   │   ├── TextInput.vue
│   │   ├── ResultsPanel.vue
│   │   ├── SettingsModal.vue
│   │   └── LoadingMask.vue
│   ├── composables/        # Vue 组合式函数
│   │   ├── useDetection.ts
│   │   ├── useProviders.ts
│   │   ├── useFileHandler.ts
│   │   └── useWindow.ts
│   ├── types/              # TypeScript 类型定义
│   └── styles/             # CSS 样式
├── package.json
├── vite.config.ts
└── legacy-python-electron/ # 旧代码存档
```

## Rust 依赖推荐

```toml
# Cargo.toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "protocol-asset"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }  # HTTP 客户端
anyhow = "1"                    # 错误处理
thiserror = "1"                 # 自定义错误
keyring = "2"                   # 凭证存储
pdf-extract = "0.7"             # PDF 解析
docx-rs = "0.4"                 # DOCX 解析
regex = "1"                     # 正则表达式
unicode-segmentation = "1"      # Unicode 文本处理
```

## 核心模块迁移对照

### 1. AI 提供商调用 (providers.py → providers.rs)

```rust
// src-tauri/src/services/providers.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

pub async fn call_glm(api_key: &str, prompt: &str) -> Result<String, anyhow::Error> {
    let client = Client::new();
    let resp = client
        .post("https://open.bigmodel.cn/api/paas/v4/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&ChatRequest { /* ... */ })
        .send()
        .await?;
    // 解析响应...
    Ok(result)
}
```

### 2. Tauri 命令 (替代 FastAPI 路由)

```rust
// src-tauri/src/api/detect.rs
#[tauri::command]
pub async fn detect_text(
    text: String,
    language: Option<String>,
    provider: String,
) -> Result<DetectResponse, String> {
    // 检测逻辑
    Ok(DetectResponse { /* ... */ })
}

// main.rs 注册命令
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_text,
            get_config,
            save_history,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
```

### 3. 前端调用 (JavaScript → TypeScript)

```typescript
// src/api/detect.ts
import { invoke } from '@tauri-apps/api/core';

interface DetectResponse {
  segments: Segment[];
  aggregation: Aggregation;
}

export async function detectText(text: string, provider: string): Promise<DetectResponse> {
  return await invoke('detect_text', { text, provider });
}
```

## 迁移步骤

### 第一阶段：项目初始化
1. 安装 Rust 和 Tauri CLI
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   cargo install create-tauri-app
   ```
2. 创建 Tauri 项目
   ```bash
   cargo create-tauri-app cheekAI --template vue-ts
   # 或 react-ts / svelte-ts
   ```

### 第二阶段：后端迁移
1. 实现数据模型 (schemas.rs)
2. 实现 AI 提供商调用 (providers.rs)
3. 实现文档预处理 (preprocess.rs)
4. 实现检测逻辑 (detection.rs)
5. 暴露 Tauri 命令

### 第三阶段：前端迁移
1. 迁移 UI 组件
2. 替换 IPC 调用为 Tauri invoke
3. 实现文件拖放上传
4. 实现结果展示

### 第四阶段：功能完善
1. 凭证安全存储 (keyring)
2. 配置持久化 (tauri-plugin-store)
3. 历史记录管理
4. 导出功能 (JSON/CSV)

## 开发命令

```bash
# 开发模式
cargo tauri dev

# 构建发布版
cargo tauri build

# 运行测试
cargo test

# 检查代码
cargo clippy
```

## 注意事项

1. **异步处理**：Rust 使用 async/await，确保 Tauri 命令标记为 `async`
2. **错误处理**：使用 `Result<T, String>` 或自定义错误类型
3. **序列化**：所有跨边界传递的数据需要实现 `Serialize`/`Deserialize`
4. **文件访问**：使用 Tauri 的文件系统 API 而非直接 std::fs
5. **中文处理**：使用 `unicode-segmentation` 处理中文分词

## 参考资源

- [Tauri 官方文档](https://tauri.app/start/)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)
- [reqwest HTTP 客户端](https://docs.rs/reqwest/)
- [serde 序列化](https://serde.rs/)
