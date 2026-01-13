const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('secure', {
  setGlmKey: async (key) => ipcRenderer.invoke('set-glm-key', key),
  minimize: () => ipcRenderer.send('window-min'),
  restore: () => ipcRenderer.send('window-restore'),
  maximize: () => ipcRenderer.send('window-maximize'),
  close: () => ipcRenderer.send('window-close')
})
