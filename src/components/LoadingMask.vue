<script setup lang="ts">
defineProps<{
  visible: boolean;
  text?: string;
}>();
</script>

<template>
  <div id="loadingMask" class="loading-mask" role="status" aria-live="polite" :style="{ display: visible ? 'flex' : 'none' }">
    <div class="spinner">
      <div class="dot"></div>
      <div class="dot"></div>
      <div class="dot"></div>
      <span id="loadingText">{{ text || '正在检测...' }}</span>
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
  gap: 12px;
  align-items: center;
  padding: 20px 28px;
  background: var(--bg-surface);
  border: 2px solid var(--border-dark);
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-lg);
  font-weight: 600;
  color: var(--text-surface-main);
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
}
</style>
