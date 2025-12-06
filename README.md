# CheekAI

CheekAI 是一个 AI 生成文本检测桌面应用，采用 FastAPI 后端和 Electron 前端构建。提供智能的文本文档分析和检测能力。

## 🏗️ 架构

**后端 (Python/FastAPI)**
- FastAPI Web 服务，用于 AI 文本检测
- 文档预处理（支持 PDF、DOCX）
- 多 AI 提供商集成（当前支持 GLM API，后续计划兼容其他 API 格式）
- 配置管理及版本控制
- 历史记录跟踪系统

**桌面端 (Electron)**
- 跨平台桌面应用
- 无边框自定义 UI
- 后端进程生命周期管理
- 安全的凭证存储（keytar + electron-store）
- 前后端 IPC 通信

## 📋 环境要求

- **Python** 3.8+ (含 pip)
- **Node.js** 16+ (含 npm)
- **Windows** (当前针对 Windows 优化)

## 🚀 快速开始

### 1. 安装 Python 依赖

```bash
pip install -r backend/requirements.txt
```

### 2. 运行应用

最简单的方式是同时启动后端和桌面端：

```bash
python start.py
```

这将会：
- 自动启动 FastAPI 后端在 `http://127.0.0.1:8787`
- 如有需要会自动安装桌面端依赖
- 启动 Electron 桌面应用
- 管理后端进程生命周期

### 备选方案：手动启动

**仅启动后端：**
```bash
python -m uvicorn backend.app.main:api --host 127.0.0.1 --port 8787
```

**仅启动桌面端：**
```bash
cd desktop
npm install  # 仅首次需要
npm run start
```

## 📁 项目结构

```
cheekAI/
├── backend/                 # FastAPI 后端
│   ├── app/
│   │   ├── core/           # 核心配置
│   │   ├── models/         # 数据模型
│   │   ├── routers/        # API 路由
│   │   │   ├── config.py   # 配置端点
│   │   │   ├── detect.py   # 检测端点
│   │   │   └── history.py  # 历史记录端点
│   │   ├── services/       # 业务逻辑服务
│   │   ├── config_store.py # 配置版本管理
│   │   ├── main.py         # FastAPI 应用入口
│   │   ├── preprocess.py   # 文档预处理
│   │   ├── providers.py    # AI 提供商集成
│   │   ├── schemas.py      # Pydantic 模式
│   │   └── service.py      # 核心检测服务
│   ├── config/
│   │   └── api_config.json # 主配置文件
│   └── requirements.txt
├── desktop/                # Electron 桌面应用
│   ├── renderer/           # 前端 UI
│   │   ├── index.html
│   │   ├── index.js
│   │   └── style.css
│   ├── main.js            # Electron 主进程
│   ├── preload.js         # 预加载脚本
│   └── package.json
├── docs/                  # 文档
├── samples/               # 示例文件
├── scripts/               # 工具脚本
├── start.py              # 统一启动脚本
└── backend_entry.py      # 后端入口点

```

## 🔧 配置

### 后端配置

配置存储在 `backend/config/api_config.json`，具有自动版本控制功能。每次配置更改都会在 `backend/config/versions/` 创建带时间戳的备份。

### 环境变量

- `CHEEKAI_BACKEND_MANAGED`: 设置为 `1` 表示后端由外部管理（由 `start.py` 使用）
- `APP_PROXY_URL`: 可选的 HTTP/HTTPS 代理 URL
- `NPM_PATH`: 自定义 npm 可执行文件路径（如果不在 PATH 中）

### 桌面端配置

桌面端设置安全存储在：
- **keytar**: 操作系统级凭证存储（Windows 凭据管理器）
- **electron-store**: 本地配置文件备用方案

## 🛠️ 开发

### 后端开发

```bash
# 安装依赖
pip install -r backend/requirements.txt

# 使用自动重载运行
python -m uvicorn backend.app.main:api --reload --host 127.0.0.1 --port 8787

# 访问 API 文档
# http://127.0.0.1:8787/docs
```

### 桌面端开发

```bash
cd desktop
npm install
npm run start
```

### 生产环境构建

```bash
cd desktop
npm run dist
```

这将在 `desktop/dist_final/` 目录创建可分发包。

## 📝 API 端点

- 自定义无边框窗口设计
- 拖放文件上传（PDF、DOCX、TXT）
- 实时检测结果
- 结构化输出视图
- 导出为 JSON/CSV
- 检测历史管理
- API 密钥配置

## 🔒 安全性

- API 密钥安全存储在 Windows 凭据管理器中
- Git 仓库中不包含敏感数据
- 后端仅通过 localhost 访问
- CORS 限制为 localhost 来源

## 📄 许可证

本项目采用 [MIT 许可证](LICENSE) 开源。

## 🤝 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建您的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交您的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启一个 Pull Request

开发规范请参考 `AGENTS.md`。
