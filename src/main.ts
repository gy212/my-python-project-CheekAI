import { createApp } from "vue";
import App from "./App.vue";
import { invoke } from "@tauri-apps/api/core";

const clientStart = performance.now();

// 创建并挂载 Vue 应用
const app = createApp(App);
app.mount("#app");

invoke("report_frontend_ready", { phase: "mounted", clientMs: performance.now() - clientStart }).catch(() => {});

// 平滑过渡：等待 DOM 更新后移除 Splash Screen
requestAnimationFrame(() => {
  invoke("report_frontend_ready", { phase: "raf", clientMs: performance.now() - clientStart }).catch(() => {});
  // 给 Vue 一些时间完成渲染
  setTimeout(() => {
    const splash = document.getElementById('splash');
    const appEl = document.getElementById('app');
    
    if (appEl) {
      appEl.classList.add('ready');
    }
    
    if (splash) {
      splash.classList.add('fade-out');
      // 动画结束后完全移除
      setTimeout(() => {
        splash.remove();
      }, 400);
    }
  }, 100);
});
