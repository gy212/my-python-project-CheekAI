/**
 * CheekAI Desktop Application - Electron Main Process
 *
 * This module manages:
 * - Backend process lifecycle
 * - Window management
 * - Credential storage (keytar + electron-store)
 * - IPC communication with renderer
 */

const { app, BrowserWindow, ipcMain, dialog } = require('electron')
const path = require('path')
const { spawn } = require('child_process')
const http = require('http')

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const _LOCAL_PROXY_BYPASS = '127.0.0.1;localhost'
app.commandLine.appendSwitch('proxy-bypass-list', _LOCAL_PROXY_BYPASS)
app.commandLine.appendSwitch('no-proxy-server', _LOCAL_PROXY_BYPASS)
app.commandLine.appendSwitch('proxy-server', 'direct://')

const MANAGED_BACKEND = process.env.CHEEKAI_BACKEND_MANAGED === '1'
const BACKEND_PORT = 8787
const BACKEND_HOST = '127.0.0.1'
const HEALTH_CHECK_INTERVAL = 3000
const BACKEND_TIMEOUT = 15000

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let secretStorePromise = null
let backendProcess = null
let backendReadyPromise = null
let healthFails = 0

// ---------------------------------------------------------------------------
// Secret Store Management
// ---------------------------------------------------------------------------

async function ensureSecretStores() {
  if (!secretStorePromise) {
    secretStorePromise = (async () => {
      try {
        const { default: Store } = await import('electron-store')
        const keytarModule = await import('keytar')
        return {
          store: new Store(),
          keytar: keytarModule.default || keytarModule
        }
      } catch (err) {
        console.error('secret_store_init_error', err)
        return { store: null, keytar: null }
      }
    })()
  }
  return secretStorePromise
}

// ---------------------------------------------------------------------------
// Backend Communication
// ---------------------------------------------------------------------------

function checkBackend() {
  return new Promise(resolve => {
    const req = http.get(`http://${BACKEND_HOST}:${BACKEND_PORT}/api/health`, res => {
      resolve(res.statusCode === 200)
    })
    req.on('error', () => resolve(false))
    req.end()
  })
}

function postBackendJson(pathname, payload) {
  return new Promise((resolve, reject) => {
    const body = JSON.stringify(payload)
    const req = http.request(
      {
        hostname: BACKEND_HOST,
        port: BACKEND_PORT,
        path: pathname,
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(body)
        }
      },
      res => {
        const chunks = []
        res.on('data', chunk => chunks.push(chunk))
        res.on('end', () => {
          const text = Buffer.concat(chunks).toString()
          if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            resolve(text)
          } else {
            reject(new Error(`HTTP ${res.statusCode}: ${text || 'unknown error'}`))
          }
        })
      }
    )
    req.on('error', reject)
    req.write(body)
    req.end()
  })
}

// ---------------------------------------------------------------------------
// Backend Process Management
// ---------------------------------------------------------------------------

async function waitForBackendReady(timeoutMs = BACKEND_TIMEOUT) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (await checkBackend()) {
      return true
    }
    await new Promise(r => setTimeout(r, 400))
    if (!backendProcess) break
  }
  return false
}

async function ensureBackend() {
  if (await checkBackend()) {
    return true
  }

  if (MANAGED_BACKEND) {
    return waitForBackendReady(20000)
  }

  if (!backendProcess) {
    if (app.isPackaged) {
      const backendPath = path.join(process.resourcesPath, 'backend', 'cheekAI_server', 'cheekAI_server.exe')
      backendProcess = spawn(backendPath, [], {
        cwd: path.dirname(backendPath),
        stdio: ['ignore', 'pipe', 'pipe']
      })
    } else {
      // Dev mode: run Python directly
      const projectRoot = path.join(__dirname, '..')
      backendProcess = spawn('python', [
        '-m', 'uvicorn', 'backend.app.main:api',
        '--host', BACKEND_HOST,
        '--port', String(BACKEND_PORT)
      ], {
        cwd: projectRoot,
        shell: true,
        stdio: ['ignore', 'pipe', 'pipe']
      })
    }

    const logBackend = data => {
      console.log(`[backend] ${data.toString().trim()}`)
    }

    if (backendProcess.stdout) backendProcess.stdout.on('data', logBackend)
    if (backendProcess.stderr) backendProcess.stderr.on('data', logBackend)

    backendProcess.on('exit', (code) => {
      console.log(`[backend] Process exited with code ${code}`)
      backendProcess = null
      backendReadyPromise = null
    })
  }

  if (!backendReadyPromise) {
    backendReadyPromise = (async () => {
      const ready = await waitForBackendReady()
      if (!ready) {
        await dialog.showMessageBox({
          type: 'error',
          title: '后端无法启动',
          message: '未能在 15 秒内连接到后端，请检查 Python 依赖或端口占用'
        }).catch(() => {})
      }
      return ready
    })()
  }

  return backendReadyPromise
}

function killBackend() {
  if (backendProcess) {
    try {
      backendProcess.kill()
    } catch (err) {
      console.error('Failed to kill backend process:', err)
    }
    backendProcess = null
  }
}

// ---------------------------------------------------------------------------
// Credential Management
// ---------------------------------------------------------------------------

async function loadStoredKey() {
  try {
    const secrets = await ensureSecretStores()
    const { store, keytar } = secrets
    const service = 'cheekai-glm'
    const account = process.env.USERNAME || 'user'

    let key = null

    // Try keytar first (OS-level secure storage)
    if (keytar && keytar.getPassword) {
      key = await keytar.getPassword(service, account)
    }

    // Fallback to electron-store
    if (!key && store) {
      try {
        key = store.get('glmKey') || null
      } catch {}
    }

    return key
  } catch (err) {
    console.error('Failed to load stored key:', err)
    return null
  }
}

async function injectStoredKey() {
  const key = await loadStoredKey()

  if (key) {
    console.log(`startup_inject_key source=local len=${key.length}`)

    // Wait for backend to be ready
    let tries = 0
    while (tries < 10) {
      if (await checkBackend()) break
      tries++
      await new Promise(r => setTimeout(r, 500))
    }

    try {
      await postBackendJson('/api/config/glm', { apiKey: key })
    } catch (err) {
      console.error('Failed to inject key to backend:', err)
    }
  }
}

// ---------------------------------------------------------------------------
// Window Management
// ---------------------------------------------------------------------------

async function createWindow() {
  const ready = await ensureBackend()
  if (!ready) {
    throw new Error('Backend failed to start')
  }

  const win = new BrowserWindow({
    width: 980,
    height: 700,
    frame: false,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  })

  await win.loadFile(path.join(__dirname, 'renderer', 'index.html'))
  return win
}

// ---------------------------------------------------------------------------
// Health Monitoring
// ---------------------------------------------------------------------------

function startHealthMonitor() {
  setInterval(async () => {
    if (MANAGED_BACKEND) return

    const ok = await checkBackend()
    if (!ok) {
      healthFails += 1
      if (healthFails >= 3) {
        console.log('[health] Backend unhealthy, restarting...')
        killBackend()
        await ensureBackend()
        healthFails = 0
      }
    } else {
      healthFails = 0
    }
  }, HEALTH_CHECK_INTERVAL)
}

// ---------------------------------------------------------------------------
// IPC Handlers
// ---------------------------------------------------------------------------

function setupIpcHandlers() {
  // GLM Key management
  ipcMain.handle('set-glm-key', async (_e, key) => {
    const secrets = await ensureSecretStores()
    const { store, keytar } = secrets
    const service = 'cheekai-glm'
    const account = process.env.USERNAME || 'user'

    // Store in keytar (secure)
    if (keytar && keytar.setPassword) {
      await keytar.setPassword(service, account, key)
    }

    // Backup to electron-store
    if (store) {
      try {
        store.set('glmKey', key)
      } catch {}
    }

    // Send to backend with retry
    let lastErr = null
    for (let i = 0; i < 3; i++) {
      try {
        await postBackendJson('/api/config/glm', { apiKey: key })
        return { ok: true }
      } catch (e) {
        lastErr = e
        await new Promise(r => setTimeout(r, 500))
      }
    }

    await dialog.showMessageBox({
      type: 'error',
      title: '保存失败',
      message: '后端未就绪或网络错误，请稍后再试',
      detail: String(lastErr || '')
    }).catch(() => {})

    return { ok: false, message: lastErr ? String(lastErr.message || lastErr) : '保存失败' }
  })

  // Window controls
  ipcMain.on('window-min', (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (win) win.minimize()
  })

  ipcMain.on('window-restore', (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (win) win.unmaximize()
  })

  ipcMain.on('window-maximize', (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (win) win.maximize()
  })

  ipcMain.on('window-close', (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (win) win.close()
  })
}

// ---------------------------------------------------------------------------
// Application Lifecycle
// ---------------------------------------------------------------------------

app.whenReady().then(async () => {
  // Setup IPC handlers
  setupIpcHandlers()

  // Start health monitoring
  startHealthMonitor()

  // Create main window
  try {
    await createWindow()
  } catch (err) {
    console.error('failed_to_create_window', err)
    app.quit()
    return
  }

  // Inject stored API key after window is created
  await injectStoredKey()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('before-quit', () => {
  killBackend()
})
