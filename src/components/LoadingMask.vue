<script setup lang="ts">
defineProps<{
  visible: boolean;
  text?: string;
  progress?: number;
}>();
</script>

<template>
  <div id="loadingMask" class="loading-mask" role="status" aria-live="polite" :style="{ display: visible ? 'flex' : 'none' }">
    <div class="spinner">
      <div class="dot"></div>
      <div class="dot"></div>
      <div class="dot"></div>
      <div class="progress-info">
        <span id="loadingText">{{ text || '正在检测...' }}</span>
        <span v-if="progress" class="progress-percent">{{ progress }}%</span>
      </div>
      <div v-if="progress" class="progress-bar">
        <div class="progress-fill" :style="{ width: progress + '%' }"></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.loading-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 70, 67, 0.7);
  backdrop-filter: blur(3px);
  display: none;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.loading-mask .spinner {
  display: flex;
  flex-direction: column;
  gap: 12px;
  align-items: center;
  padding: 20px 28px;
  background: var(--bg-surface);
  border: 2px solid var(--border-dark);
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-lg);
  min-width: 200px;
}

.loading-mask .spinner > .dot {
  display: none;
}

.loading-mask .spinner:has(.progress-bar) > .dot {
  display: none;
}

.loading-mask .spinner:not(:has(.progress-bar)) {
  flex-direction: row;
}

.loading-mask .spinner:not(:has(.progress-bar)) > .dot {
  display: block;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  width: 100%;
  font-weight: 600;
  color: var(--text-surface-main);
}

.progress-percent {
  color: var(--primary);
}

.progress-bar {
  width: 100%;
  height: 6px;
  background: var(--border-dark);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--primary);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.loading-mask .dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--primary);
  animation: dot-bounce 1s infinite ease-in-out;
}

.loading-mask .dot:nth-child(2) { animation-delay: 0.1s; }
.loading-mask .dot:nth-child(3) { animation-delay: 0.2s; }

@keyframes dot-bounce {
  0%, 80%, 100% { transform: scale(0.8); opacity: 0.4; }
  40% { transform: scale(1.1); opacity: 1; }
}

@media (prefers-reduced-motion: reduce) {
  .loading-mask .dot {
    animation: none;
    opacity: 1;
  }
  .progress-fill {
    transition: none;
  }
}
</style>
