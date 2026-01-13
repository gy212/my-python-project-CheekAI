// Window Control Composable
// Handles Tauri window operations

import { getCurrentWindow } from "@tauri-apps/api/window";

export function useWindow() {
  const appWindow = getCurrentWindow();

  async function startDrag() {
    await appWindow.startDragging();
  }

  async function minimizeWindow() {
    await appWindow.minimize();
  }

  async function toggleMaximizeWindow() {
    await appWindow.toggleMaximize();
  }

  async function closeWindow() {
    await appWindow.close();
  }

  return {
    startDrag,
    minimizeWindow,
    toggleMaximizeWindow,
    closeWindow,
  };
}
