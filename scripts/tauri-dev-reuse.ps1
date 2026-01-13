$ErrorActionPreference = "Stop"

# 复用已启动的 Vite 开发服务器，跳过 Tauri 的 beforeDevCommand
$devUrl = "http://127.0.0.1:1420"

Write-Host "提示：先在另一个终端运行 Vite（保持常驻）：" -ForegroundColor Yellow
Write-Host "  npm run dev -- --host 127.0.0.1 --port 1420" -ForegroundColor Yellow

# 检查端口是否已监听
$check = Test-NetConnection -ComputerName 127.0.0.1 -Port 1420 -WarningAction SilentlyContinue
if (-not $check.TcpTestSucceeded) {
  Write-Warning "检测到 1420 端口未监听，先启动 Vite 再重试。"
  exit 1
}

$env:TAURI_SKIP_DEVSERVER = "1"
$env:TAURI_DEV_URL = $devUrl

Write-Host "已设置 TAURI_SKIP_DEVSERVER，复用 $devUrl，启动 Tauri..." -ForegroundColor Cyan
npm run tauri -- dev
