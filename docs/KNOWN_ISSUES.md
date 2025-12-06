## 现存问题与注意事项

- **GLM 调用需要有效密钥**：`/api/detect` 现在会强制调用 GLM（若存在密钥），调用失败会返回 502 而不是静默回退本地启发式。请确认密钥有效、外网可达，否则前端会弹错。
- **调用耗时与日志**：后端会记录 `detect_request`、GLM 请求耗时与状态；`cost.providerBreakdown` 返回 `glmRequested/glmSuccess`。若仍出现“秒出结果”，请检查后端日志是否有 GLM 失败。
- **测试用例未执行**：当前 `backend/tests` 文件名以时间戳开头，`pytest` 0 个用例被收集；如需自动执行请将文件名改为 `test_*.py` 或在 CI 中指定 `-p no:warnings -q 2025*_test.py`。
- **Electron 开发模式警告**：CSP 警告仅在开发模式出现，打包后消失；如需消除请在 `index.html` 设置严格 CSP。
- **依赖环境**：已新建 `.venv` 并安装 `backend/requirements.txt`。启动前请激活虚拟环境运行 `python start.py`，Electron 由 start.py 或 `npm run start` 自动安装依赖。
- **端口占用保护**：`start.py` 会设置 `CHEEKAI_BACKEND_MANAGED=1`，Electron 不再自行拉起/重启后端；若手动单独启动桌面端注意不要再次启动后端以免占用 8787 端口。
