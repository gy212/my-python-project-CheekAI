<script setup lang="ts">
defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

function handleInput(event: Event) {
  const target = event.target as HTMLTextAreaElement;
  emit("update:modelValue", target.value);
}
</script>

<template>
  <article class="surface card input-card">
    <header class="card-heading">
      <p class="eyebrow">待检测文本</p>
      <h2>粘贴或上传内容</h2>
    </header>
    <textarea 
      id="inputText" 
      placeholder="输入或粘贴待检测文本..." 
      :value="modelValue"
      @input="handleInput"
    ></textarea>
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

.card-heading h2 {
  margin: 4px 0 0;
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--text-surface-main);
}

textarea {
  width: 100%;
  min-height: 260px;
  border-radius: var(--radius-card);
  padding: 18px;
  border: 2px solid var(--border-dark);
  background: var(--bg-input);
  font-size: var(--font-base);
  line-height: 1.6;
  resize: vertical;
  color: var(--text-dark);
  box-shadow: var(--shadow-lg);
  transition: all var(--transition-fast);
  font-family: inherit;
}

textarea:hover {
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-xl);
}

textarea:focus {
  outline: none;
  border-color: var(--primary);
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-xl);
}

textarea::placeholder {
  color: var(--text-surface-muted);
}
</style>
