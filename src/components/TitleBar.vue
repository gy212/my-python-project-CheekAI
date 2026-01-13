<script setup lang="ts">
import { useWindow } from "@/composables";

const { startDrag, minimizeWindow, toggleMaximizeWindow, closeWindow } = useWindow();
</script>

<template>
  <div id="title-bar">
    <div class="drag-region" @mousedown="startDrag">CheekAI</div>
    <div class="window-controls">
      <button id="btn-min" class="win-control" title="Minimize" @click="minimizeWindow">
        <svg width="10" height="1" viewBox="0 0 10 1"><path d="M0 0h10v1H0z" fill="currentColor"/></svg>
      </button>
      <button id="btn-restore" class="win-control" title="Restore" @click="toggleMaximizeWindow">
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M3 3v7h7V3H3zm6 6H4V4h5v5zM1 1v7h1V2h7V1H1z" fill="currentColor"/></svg>
      </button>
      <button id="btn-close" class="win-control" title="Close" @click="closeWindow">
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 0L0 1l4 4-4 4 1 1 4-4 4 4 1-1-4-4 4-4-1-1-4 4-4-4z" fill="currentColor"/></svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
#title-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 32px;
  background: var(--bg-app);
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 10001;
}

.drag-region {
  padding-left: 16px;
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 600;
  display: flex;
  align-items: center;
  flex: 1;
  height: 100%;
}

.window-controls {
  display: flex;
}

.win-control {
  width: 46px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  outline: none;
  transition: all var(--transition-fast);
}

.win-control:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: var(--text-main);
}

.win-control:focus-visible {
  background-color: rgba(255, 255, 255, 0.15);
  outline: none;
}

#btn-close:hover {
  background-color: var(--accent);
  color: white;
}
</style>
