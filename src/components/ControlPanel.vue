<script setup lang="ts">
import { ref, computed } from "vue";
import { SENSITIVITY_OPTIONS } from "@/types";
import type { ProviderOption } from "@/types";

const props = defineProps<{
  sensitivity: string;
  selectedProvider: string;
  providerOptions: ProviderOption[];
  fileName: string;
  dualMode: boolean;
}>();

const emit = defineEmits<{
  (e: "update:sensitivity", value: string): void;
  (e: "update:selectedProvider", value: string): void;
  (e: "update:dualMode", value: boolean): void;
  (e: "detect"): void;
  (e: "open-settings"): void;
  (e: "trigger-file-select"): void;
}>();

const sensitivityOpen = ref(false);
const providerOpen = ref(false);

const currentSensitivityLabel = computed(() => 
  SENSITIVITY_OPTIONS.find(o => o.value === props.sensitivity)?.label || props.sensitivity
);

const currentProviderLabel = computed(() => {
  if (!props.selectedProvider) return "任意 LLM 判别";
  const found = props.providerOptions.find(o => o.value === props.selectedProvider);
  return found ? found.label : props.selectedProvider;
});

function selectSensitivity(val: string) {
  emit("update:sensitivity", val);
  sensitivityOpen.value = false;
}

function selectProvider(val: string) {
  emit("update:selectedProvider", val);
  providerOpen.value = false;
}

function closeSelects(e: Event) {
  const target = e.target as HTMLElement;
  if (!target.closest('.custom-select-container')) {
    sensitivityOpen.value = false;
    providerOpen.value = false;
  }
}

// Setup click outside listener
import { onMounted, onUnmounted } from "vue";
onMounted(() => {
  document.addEventListener('click', closeSelects);
});
onUnmounted(() => {
  document.removeEventListener('click', closeSelects);
});
</script>

<template>
  <article class="surface card control-card">
    <header class="card-heading">
      <p class="eyebrow">检测控制</p>
      <h1>运行参数</h1>
      <p class="card-subtitle">灵敏度影响判定阈值 · LLM 提供商管理</p>
    </header>
    <div class="form-grid">
      <!-- Sensitivity Select -->
      <div class="form-field">
        <label class="form-label">敏感度</label>
        <div class="custom-select-container">
          <div 
            class="custom-select-trigger" 
            :class="{ open: sensitivityOpen }" 
            @click.stop="sensitivityOpen = !sensitivityOpen; providerOpen = false"
          >
            <span class="custom-select-value">{{ currentSensitivityLabel }}</span>
            <div class="arrow-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>
            </div>
          </div>
          <div class="custom-select-options" :class="{ open: sensitivityOpen }">
            <div 
              v-for="opt in SENSITIVITY_OPTIONS" 
              :key="opt.value" 
              class="custom-select-option" 
              :class="{ selected: sensitivity === opt.value }" 
              @click="selectSensitivity(opt.value)"
            >
              {{ opt.label }}
            </div>
          </div>
        </div>
        <p class="form-hint">灵敏度只影响判定阈值与复核策略，不改变风险概率</p>
      </div>
      
      <!-- Provider Select -->
      <div class="form-field">
        <label class="form-label">判别模型</label>
        <div class="custom-select-container">
          <div 
            class="custom-select-trigger" 
            :class="{ open: providerOpen }" 
            @click.stop="providerOpen = !providerOpen; sensitivityOpen = false"
          >
            <span class="custom-select-value">{{ currentProviderLabel }}</span>
            <div class="arrow-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>
            </div>
          </div>
          <div class="custom-select-options" :class="{ open: providerOpen }">
            <div 
              class="custom-select-option" 
              :class="{ selected: selectedProvider === '' }" 
              @click="selectProvider('')"
            >
              任意 LLM 判别
            </div>
            <div 
              v-for="p in providerOptions" 
              :key="p.value" 
              class="custom-select-option" 
              :class="{ selected: selectedProvider === p.value }" 
              @click="selectProvider(p.value)"
            >
              {{ p.label }}
            </div>
          </div>
        </div>
      </div>
      
      <!-- File Picker -->
      <div class="form-field span-2 file-field">
        <span class="form-label">文本预处理</span>
        <div class="file-picker">
          <button id="fileTrigger" class="btn-secondary" type="button" @click="$emit('trigger-file-select')">选择文件</button>
          <span id="fileName" class="file-name">{{ fileName }}</span>
        </div>
      </div>
      
      <!-- Dual Mode Checkbox -->
      <div class="form-field span-2">
        <label class="form-label" style="display:flex;align-items:center;gap:8px;cursor:pointer">
          <input type="checkbox" :checked="dualMode" @change="$emit('update:dualMode', ($event.target as HTMLInputElement).checked)" />
          <span>双模式检测 (段落+句子)</span>
        </label>
      </div>
      
      <!-- Action Buttons -->
      <div class="form-actions span-2 action-row">
        <button id="detectButton" class="btn-primary" type="button" @click="$emit('detect')">开始检测</button>
        <button class="btn-secondary" type="button" @click="$emit('open-settings')">设置</button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.card {
  padding: 24px;
}

.card-heading {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 20px;
  border-bottom: 1px dashed var(--border);
  padding-bottom: 12px;
}

.card-heading h1 {
  margin: 4px 0 0;
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--text-surface-main);
}

.card-subtitle {
  margin: 0;
  font-size: var(--font-sm);
  color: var(--text-surface-muted);
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-label {
  font-size: var(--font-xs);
  text-transform: uppercase;
  letter-spacing: 0.1em;
  font-weight: 700;
  color: var(--text-surface-muted);
}

.form-hint {
  margin: 0;
  font-size: var(--font-xs);
  color: var(--text-muted);
}

.span-2 {
  grid-column: span 2;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  flex-wrap: wrap;
  padding-top: 4px;
}

.action-row .btn-secondary,
.action-row .btn-primary {
  min-width: 120px;
}

.file-picker {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.file-name {
  font-size: var(--font-sm);
  color: var(--text-surface-muted);
  font-style: italic;
}

/* Custom Select Styles */
.custom-select-container {
  position: relative;
  width: 100%;
  font-family: inherit;
}

.custom-select-trigger {
  width: 100%;
  height: 44px;
  border-radius: var(--radius-sm);
  border: 2px solid var(--border-dark);
  padding: 10px 14px;
  font-size: var(--font-base);
  font-weight: 500;
  background: var(--bg-input);
  color: var(--text-dark);
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  box-shadow: var(--shadow-md);
  transition: all var(--transition-fast);
  user-select: none;
}

.custom-select-trigger:hover {
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-lg);
}

.custom-select-trigger.open {
  border-color: var(--primary);
}

.custom-select-options {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  right: 0;
  background: var(--bg-input);
  border: 2px solid var(--border-dark);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-lg);
  z-index: 1000;
  max-height: 240px;
  overflow-y: auto;
  opacity: 0;
  visibility: hidden;
  transform: translateY(-8px);
  transition: all var(--transition-normal);
}

.custom-select-options.open {
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
}

.custom-select-option {
  padding: 10px 14px;
  cursor: pointer;
  transition: background var(--transition-fast);
  color: var(--text-dark);
  font-weight: 500;
}

.custom-select-option:hover {
  background: var(--secondary);
}

.custom-select-option.selected {
  background: var(--primary);
  color: var(--text-dark);
}

.arrow-icon {
  width: 16px;
  height: 16px;
  transition: transform var(--transition-fast);
  color: var(--text-dark);
}

.custom-select-trigger.open .arrow-icon {
  transform: rotate(180deg);
}

/* Checkbox styling */
input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--primary);
  cursor: pointer;
}
</style>
