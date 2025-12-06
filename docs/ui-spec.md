# CheekAI 桌面端 UI 设计规范（现代简约、淡雅清新）

## 视觉风格
- 现代简约与优雅：强调留白与层次，克制装饰，突出内容可读性。
- 圆角：组件 12–14px；卡片 16px；按钮 12px。
- 阴影：卡片 `var(--shadow)`、悬浮 `var(--shadow-soft)`；浅层级，避免强对比。
- 半透明与毛玻璃：顶栏与容器使用 `background: rgba(255,255,255,0.6)` 与 `backdrop-filter: blur(12–14px)`。

## 色彩系统（≤3 主色，低饱和度）
- 主色（Primary）：`#AEE1D0`（柔和浅青绿）— 主要操作与品牌识别。
- 辅助（Secondary）：`#D5CFE1`（淡雅浅紫）— 次级强调与装饰性背景。
- 强调（Accent）：`#A8C5FF`（柔和浅蓝）— 焦点、链接与交互反馈。
- 中性色：`#111827` / `#374151` / `#6B7280` / `#D1D5DB` / `#E5E7EB` / `#F3F4F6`。
- 语义色：成功 `#9AD4B0`、警告 `#F6D48F`、错误 `#E7A0A0`（低饱和）。
- 使用比例：主色 60%、辅助 30%、强调 ≤10%；背景以中性浅灰为主。

## 字体系统
- 中文主字体：Noto Sans SC；英文/数字：Inter；回退 `system-ui, Segoe UI`。
- 层级字号：12/14/16/20/24/32/40；正文 14–16；行高 1.5–1.7；字重 400/500/600。

## 排版与栅格
- 布局：双栏（左 380px 控件与文本，右 1fr 结果与批量）；窄屏自适应单列。
- 断点：≥1440（多列宽屏）、≥1200（两列主从）、≥960（单列+可折叠侧区）、<960（单列）。
- 间距：8px 基准（4/8/12/16/24/32/48/64）统一控制；容器背景 `var(--neutral-100)`。

## 组件规范
- 按钮：高度 36px，圆角 12px；样式分层：`btn-primary`（主色填充）、`btn-secondary`（浅灰背景）、`btn-link`（强调色文本）。
- 输入：聚焦边框 `var(--accent)`；3px 内发光 `rgba(168,197,255,0.28)`；高度统一。
- 卡片：圆角 16px；内部留白 14px；分段结果与批量项使用同体系。
- Pill：使用 `var(--secondary)` 背景与中性文本；用于状态与指标简报。

## 动效规范
- 过渡：120–250ms ease；入场动画 `reveal 240ms`；减少运动时全部关闭。
- 悬停与聚焦：柔和边框与阴影增强；按压轻微缩放（0.98）。

## 图标与视觉元素
- 图标：线性 2px 描边、圆角端点、尺寸 16–20px；配色随主题。
- 插画：轻量几何/线稿，采用主/辅色的低饱和变体。

## 页面与信息架构
- 总览（Dashboard）：系统状态、快捷入口、最近任务。
- 模型设置：敏感度、提供商与密钥管理（含状态反馈）。
- 检测中心：单文件/批量任务、进度与结果概览。
- 预处理中心：清洗规则、预处理队列、历史记录。
- 结果查看：结构映射树、分段预览、文本输入与高亮。
- 设置：主题、隐私与日志、网络与更新。

## 响应式设计方案
- 重排策略：优先折叠次要区域与栅格重排，保持操作可达与视觉舒适。
- Electron 窗口缩放下栅格与间距按断点适配。

## 设计交付物
- UI 设计规范文档（本文件：排版、色彩、栅格、组件、动效、交互）。
- 高保真页面设计稿（桌面断点 ≥1200 与 ≥1440）。
- 色彩系统说明（主/辅/强调/语义/中性，含比例与用法）。
- 响应式重排规则与示例。
- 设计源文件：Figma（可按需提供 Sketch/XD 导出）。
- 设计 Token：`desktop/renderer/design-tokens.json` 与 `style.css :root` 映射。

## 集成说明（frontend → desktop/renderer）
- 资源合并：
  - 设计 Token：已对齐 `frontend/design-tokens.json` 至 `desktop/renderer/design-tokens.json`，补充 `gradient/animation` 字段并统一命名层级。
  - 样式模块：已合并 `frontend/style.css` 的 `status-bar/top-actions/main-card/dest-grid/bottom-nav/progress-ring` 至 `desktop/renderer/style.css`。
- 页面结构：
  - 在 `desktop/renderer/index.html` 引入 `status-bar/top-actions/bottom-nav`，保留原有业务双栏布局与控件区。
  - 结果卡片加入环形进度组件，根据总体概率动态渲染。
- 交互映射：
  - 进度环：`aggregation.overallProbability` → 百分比；渲染逻辑位于 `desktop/renderer/index.js`。
  - 底部导航：按钮状态切换（不影响业务路由）。
- 启动与后端：
  - 启动入口不变：`start.py`（后端健康检查与进程管理）、`desktop/main.js`（渲染入口与后端自启）。

## 文件参考
- 渲染入口：`desktop/renderer/index.html`
- 样式与变量：`desktop/renderer/style.css`、`desktop/renderer/design-tokens.json`
- 结果渲染与交互：`desktop/renderer/index.js`
- 启动脚本：`start.py`
- 后端 API：`backend/app/main.py`

## 验收与适配
- 视觉精致优雅、淡雅配色、留白舒适；交互反馈明确一致。
- Token 完整且可落地；对接现有代码不破坏逻辑与 API 调用。

## 术语与参考
- Google Material（色彩与动效参考，取其克制）
- Fluent Design（中性色层级与留白参考）
