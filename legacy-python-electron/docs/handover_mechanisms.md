# 项目机制与算法交接说明

## 启动与运行
- 推荐入口 `python start.py`：先拉起 FastAPI（强制 `--log-level info`），轮询 `/api/health` 成功后再启动 Electron。若 20 秒未就绪会报错，请看后端日志。
- 仅后端：`python -m uvicorn backend.app.main:api --reload --log-level info`。
- 仅桌面：`cd desktop && npm run start`（默认后端 8787）。
- `NO_PROXY` 已覆盖 localhost，避免本地代理干扰。

## 前端关键点（desktop/renderer/index.js）
- 预处理完成后自动触发 `detect()`，日志标记 `[preprocess] auto trigger detect`、`[detect] start|success`。
- 检测期间有全屏 loading 遮罩，禁用上传/按钮，避免“秒出结果”错觉。
- GLM Key 通过 `/api/config/glm` 读写，`/api/providers` 下拉可选模型，IPC `window.secure.setGlmKey` 注入。

## 后端模块概览
- `backend/app/main.py`：FastAPI 路由，记录 `detect_request len=... providers=...`，异常包装为 502。
- `backend/app/service.py`：预处理、分段、风格学/困惑度评估、LLM 判别、聚合。
- `backend/app/providers.py`：GLM4.6 调用封装，response_format=json_object，写入 `backend/logs/glm_last_response.json`。

## 分段机制
1) 构建 block：`buildParagraphBlocksFromText`/nodes 生成含 `label` 的段落，正文为 `label=body`。
2) **默认 LLM 分段** `_build_segments_via_llm`：
   - 只把 `label=body` 列表传给 GLM，非正文直接丢弃。
   - 规则：保持顺序，只合并相邻；必须覆盖所有正文；段数/长度完全自由；仅输出 JSON `{"segments":[{"chunk_id":0,"paragraph_start":0,"paragraph_end":2}]}`。
   - 解析失败会尝试正则抽取 `{...}`；仍失败则返回 None。
3) **本地回退**：若 LLM 失败，则 `buildSegmentsAligned` 按敏感度的 chunk size（高敏≈360 token，含重叠）切分，必要时同一 block 多切。
4) 日志：
   - `segmentation_profile chunk=... overlap=... blocks=... len=...`
   - `segmentation_result segments=N source=llm|local`
   - 若 LLM 返回异常，查看 `backend/logs/glm_last_response.json`。

## 检测算法
- `computeStylometryMetrics`：词汇多样性(TTR)、平均句长、功能词密度、标点密度、重复率、n-gram 重复率等。
- `estimateTokens`：将中文逐字计为 token，避免整段当 1 个。
- `_score_segment`：启发式结合 stylometry + 粗略困惑度得出 `aiProbability`。
- `_run_llm_judgment`：把每段指标打包为 JSON 交给 GLM 判别，返回 prob 与本地 0.7/0.3 融合；若全部失败则抛错。
- 聚合：`_contrast_sharpen_segments` 对比度强化，`aggregateSegments` 汇总概率/置信度，`deriveDecision` 输出结论。

## 配置与密钥
- 配置由 `backend/config_store.py` 持久化至 JSON，接口 `/api/config/file`。
- GLM Key 来源：`/api/config/glm`、环境变量 `GLM_API_KEY`/`CHEEKAI_GLM_API_KEY`，启动时自动注入。
- 默认 provider 已改为 `glm:glm-4.6`，仍兼容旧 DeepSeek 读取但不再暴露。

## 调试与日志
- 确认分段来源：看 `segmentation_result` 的 `source` 或响应中的 `cost.segmentationSource`。
- GLM 调用问题：
  - `glm_call_http_error`/`glm_call_invalid_json`/`glm_call_missing_content` 日志；
  - 原始响应：`backend/logs/glm_last_response.json`。
- 按钮无响应/秒出：检查前端控制台是否有 JS 报错，确认加载到最新 bundle。

## 测试
- 用户曾提到 `python -m pytest backend/tests/test_paragraph_blocks.py`，当前仓库无此文件；如需覆盖分段逻辑请新建 pytest。
- 现有回归脚本位于 `backend/tests/20251115T204934_*`（自带样例输入）。

## 交接建议
- 改动分段/判别提示词时同步更新本说明，保持 JSON-only 要求。
- 如需再次约束输出格式，可利用 GLM response_format 或在解析前做严格 JSON 校验。
- 若用户希望更多段落，降低 chunk size 或让 LLM 直接自由分段即可；非正文已默认忽略。
