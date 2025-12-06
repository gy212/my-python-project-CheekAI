
import os
import re
import sys
import time
import subprocess
import shutil
from urllib.request import urlopen
from urllib.error import URLError

ROOT = os.path.abspath(os.path.dirname(__file__))
BACKEND_DIR = os.path.join(ROOT, 'backend')
DESKTOP_DIR = os.path.join(ROOT, 'desktop')
HOST = '127.0.0.1'
PORT = '8787'
HEALTH_URL = f'http://{HOST}:{PORT}/api/health'
PROXY_URL = os.environ.get('APP_PROXY_URL')


def run(cmd, cwd=None, env=None):
    return subprocess.Popen(cmd, cwd=cwd or ROOT, env=env or os.environ.copy())


def wait_health(url, timeout=40, proc=None):
    start = time.time()
    while time.time() - start < timeout:
        if proc and proc.poll() is not None:
            return False
        try:
            with urlopen(url, timeout=3) as r:
                if r.status == 200:
                    return True
        except URLError:
            time.sleep(0.8)
    return False


def ensure_tools():
    if not shutil.which('node'):
        raise RuntimeError('未检测到 node，可在系统 PATH 中添加 node.exe 或安装 Node.js')
    if not shutil.which('npm'):
        raise RuntimeError('未检测到 npm，请确认 npm 已安装并加入 PATH')


def _ensure_local_no_proxy(env):
    existing = env.get('NO_PROXY') or env.get('no_proxy') or ''
    entries = [part.strip() for part in re.split(r'[\s,;]+', existing) if part.strip()]
    if not entries:
        entries = ['127.0.0.1', 'localhost']
    else:
        for host in ('127.0.0.1', 'localhost'):
            if host not in entries:
                entries.append(host)
    val = ','.join(entries)
    env['NO_PROXY'] = val
    env['no_proxy'] = val
    os.environ['NO_PROXY'] = val
    os.environ['no_proxy'] = val


def _find_npm():
    return os.environ.get('NPM_PATH') or shutil.which('npm') or shutil.which('npm.cmd') or shutil.which('npm.exe')


def ensure_desktop_dependencies(env, npm_path):
    node_modules = os.path.join(DESKTOP_DIR, 'node_modules')
    electron_exe = os.path.join(node_modules, 'electron', 'dist', 'electron.exe')
    electron_bin = os.path.join(node_modules, '.bin', 'electron.exe')
    if os.path.exists(electron_exe) or os.path.exists(electron_bin):
        return
    if not npm_path:
        raise RuntimeError('缺少 electron，且未找到 npm，请先安装 Node/npm 后重试')
    print('检测到 desktop 依赖缺失，正在执行 npm install ...', flush=True)
    proc = subprocess.run([npm_path, 'install'], cwd=DESKTOP_DIR, env=env or os.environ.copy())
    if proc.returncode != 0:
        raise RuntimeError('npm install 执行失败，请检查上方日志输出')


def start_backend(env):
    return run(
        [
            sys.executable,
            '-m',
            'uvicorn',
            'backend.app.main:api',
            '--host',
            HOST,
            '--port',
            PORT,
            '--log-level',
            'info',
        ],
        cwd=ROOT,
        env=env,
    )


def start_desktop(env):
    npm = _find_npm()
    ensure_desktop_dependencies(env, npm)
    if npm:
        return run([npm, 'run', 'start'], cwd=DESKTOP_DIR, env=env)
    electron_exe = os.path.join(DESKTOP_DIR, 'node_modules', 'electron', 'dist', 'electron.exe')
    if os.path.exists(electron_exe):
        return run([electron_exe, '.'], cwd=DESKTOP_DIR, env=env)
    raise RuntimeError('未找到 npm 或本地 electron，可设置 NPM_PATH 或先进入 desktop 执行 npm install')


def main():
    env = os.environ.copy()
    if PROXY_URL:
        env['HTTP_PROXY'] = PROXY_URL
        env['HTTPS_PROXY'] = PROXY_URL
    env['CHEEKAI_BACKEND_MANAGED'] = '1'
    _ensure_local_no_proxy(env)
    ensure_tools()
    backend = None
    ok = wait_health(HEALTH_URL, timeout=3)
    if not ok:
        backend = start_backend(env)
        print("已启动后端，等待服务就绪...", flush=True)
        ready = wait_health(HEALTH_URL, timeout=20, proc=backend)
        if not ready:
            backend.terminate()
            raise RuntimeError('后端在 20 秒内未能启动，请查看 uvicorn 日志')
    desktop = start_desktop(env)
    try:
        while True:
            if desktop.poll() is not None:
                if desktop.returncode not in (0, None):
                    raise RuntimeError('desktop 进程已退出，请检查上方日志以了解原因')
                break
            time.sleep(1)
    except KeyboardInterrupt:
        pass
    finally:
        try:
            if backend:
                backend.terminate()
        except Exception:
            pass


if __name__ == '__main__':
    main()
