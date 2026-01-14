<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save-key", provider: string, key: string): void;
}>();

const glmKey = ref("");
const deepseekKey = ref("");
const anthropicKey = ref("");
const openaiKey = ref("");
const geminiKey = ref("");
const glmStatus = ref<{ configured: boolean; preview: string }>({ configured: false, preview: "" });
const deepseekStatus = ref<{ configured: boolean; preview: string }>({ configured: false, preview: "" });
const anthropicStatus = ref<{ configured: boolean; preview: string }>({ configured: false, preview: "" });
const openaiStatus = ref<{ configured: boolean; preview: string }>({ configured: false, preview: "" });
const geminiStatus = ref<{ configured: boolean; preview: string }>({ configured: false, preview: "" });
const loading = ref(false);
const glmTestResult = ref("");
const deepseekTestResult = ref("");
const anthropicTestResult = ref("");
const openaiTestResult = ref("");
const geminiTestResult = ref("");
const testing = ref({ glm: false, deepseek: false, anthropic: false, openai: false, gemini: false });

type GptConcurrencyBenchmarkResult = {
  provider: string;
  model: string;
  concurrency: number;
  fileName: string;
  extractedChars: number;
  promptChars: number;
  totalMs: number;
  success: number;
  failed: number;
  minMs?: number;
  avgMs?: number;
  maxMs?: number;
  runs: Array<{
    index: number;
    ok: boolean;
    latencyMs?: number;
    preview?: string;
    error?: string;
  }>;
};

const openaiBenchmarkFileInput = ref<HTMLInputElement | null>(null);
const openaiBenchmarkFile = ref<File | null>(null);
const openaiBenchmarkFileName = ref("未选择文件");
const openaiBenchmarkRunning = ref(false);
const openaiBenchmarkResult = ref<GptConcurrencyBenchmarkResult | null>(null);
const openaiBenchmarkError = ref("");

// Load API key status when modal opens
watch(() => props.visible, async (visible) => {
  if (visible) {
    await loadKeyStatus();
  }
});

async function loadKeyStatus() {
  loading.value = true;
  try {
    // Check GLM key
    const glmResult = await invoke("get_api_key", { provider: "glm" }) as string | null;
    console.log("[Settings] GLM key result:", glmResult ? glmResult.slice(0, 10) + "..." : "null");
    if (glmResult && glmResult.length > 0) {
      glmStatus.value = {
        configured: true,
        preview: glmResult.slice(0, 8) + "..." + glmResult.slice(-4)
      };
    } else {
      glmStatus.value = { configured: false, preview: "" };
    }

    // Check DeepSeek key
    const deepseekResult = await invoke("get_api_key", { provider: "deepseek" }) as string | null;
    console.log("[Settings] DeepSeek key result:", deepseekResult ? deepseekResult.slice(0, 10) + "..." : "null");
    if (deepseekResult && deepseekResult.length > 0) {
      deepseekStatus.value = {
        configured: true,
        preview: deepseekResult.slice(0, 8) + "..." + deepseekResult.slice(-4)
      };
    } else {
      deepseekStatus.value = { configured: false, preview: "" };
    }

    // Check Anthropic key
    const anthropicResult = await invoke("get_api_key", { provider: "anthropic" }) as string | null;
    console.log("[Settings] Anthropic key result:", anthropicResult ? anthropicResult.slice(0, 10) + "..." : "null");
    if (anthropicResult && anthropicResult.length > 0) {
      anthropicStatus.value = {
        configured: true,
        preview: anthropicResult.slice(0, 8) + "..." + anthropicResult.slice(-4)
      };
    } else {
      anthropicStatus.value = { configured: false, preview: "" };
    }

    console.log("[Settings] Final status - GLM:", glmStatus.value, "DeepSeek:", deepseekStatus.value, "Anthropic:", anthropicStatus.value);

    // Check OpenAI key
    const openaiResult = await invoke("get_api_key", { provider: "openai" }) as string | null;
    if (openaiResult && openaiResult.length > 0) {
      openaiStatus.value = {
        configured: true,
        preview: openaiResult.slice(0, 8) + "..." + openaiResult.slice(-4)
      };
    } else {
      openaiStatus.value = { configured: false, preview: "" };
    }

    // Check Gemini key
    const geminiResult = await invoke("get_api_key", { provider: "gemini" }) as string | null;
    if (geminiResult && geminiResult.length > 0) {
      geminiStatus.value = {
        configured: true,
        preview: geminiResult.slice(0, 8) + "..." + geminiResult.slice(-4)
      };
    } else {
      geminiStatus.value = { configured: false, preview: "" };
    }
  } catch (err) {
    console.error("Failed to load API key status:", err);
  } finally {
    loading.value = false;
  }
}

function handleClose() {
  emit("close");
}

async function saveGlmKey() {
  if (!glmKey.value.trim()) {
    alert("请输入 API Key");
    return;
  }
  emit("save-key", "glm", glmKey.value);
  glmKey.value = "";
  await loadKeyStatus();
}

async function saveDeepseekKey() {
  if (!deepseekKey.value.trim()) {
    alert("请输入 API Key");
    return;
  }
  emit("save-key", "deepseek", deepseekKey.value);
  deepseekKey.value = "";
  await loadKeyStatus();
}

async function deleteGlmKey() {
  if (!confirm("确定要删除 GLM API Key 吗？")) return;
  try {
    await invoke("delete_api_key", { provider: "glm" });
    await loadKeyStatus();
  } catch (err) {
    console.error("Failed to delete GLM key:", err);
  }
}

async function deleteDeepseekKey() {
  if (!confirm("确定要删除 DeepSeek API Key 吗？")) return;
  try {
    await invoke("delete_api_key", { provider: "deepseek" });
    await loadKeyStatus();
  } catch (err) {
    console.error("Failed to delete DeepSeek key:", err);
  }
}

async function saveAnthropicKey() {
  if (!anthropicKey.value.trim()) {
    alert("请输入 API Key");
    return;
  }
  emit("save-key", "anthropic", anthropicKey.value);
  anthropicKey.value = "";
  await loadKeyStatus();
}

async function deleteAnthropicKey() {
  if (!confirm("确定要删除 Claude API Key 吗？")) return;
  try {
    await invoke("delete_api_key", { provider: "anthropic" });
    await loadKeyStatus();
  } catch (err) {
    console.error("Failed to delete Anthropic key:", err);
  }
}

async function saveOpenaiKey() {
  if (!openaiKey.value.trim()) {
    alert("请输入 API Key");
    return;
  }
  emit("save-key", "openai", openaiKey.value);
  openaiKey.value = "";
  await loadKeyStatus();
}

async function deleteOpenaiKey() {
  if (!confirm("确定要删除 OpenAI API Key 吗？")) return;
  try {
    await invoke("delete_api_key", { provider: "openai" });
    await loadKeyStatus();
  } catch (err) {
    console.error("Failed to delete OpenAI key:", err);
  }
}

async function saveGeminiKey() {
  if (!geminiKey.value.trim()) {
    alert("请输入 API Key");
    return;
  }
  emit("save-key", "gemini", geminiKey.value);
  geminiKey.value = "";
  await loadKeyStatus();
}

async function deleteGeminiKey() {
  if (!confirm("确定要删除 Gemini API Key 吗？")) return;
  try {
    await invoke("delete_api_key", { provider: "gemini" });
    await loadKeyStatus();
  } catch (err) {
    console.error("Failed to delete Gemini key:", err);
  }
}

async function testGlmConnection() {
  testing.value.glm = true;
  glmTestResult.value = "正在测试...";
  try {
    const result = await invoke("test_api_connection", { provider: "glm" }) as string;
    glmTestResult.value = result;
  } catch (err: any) {
    glmTestResult.value = typeof err === 'string' ? err : (err.message || JSON.stringify(err));
  } finally {
    testing.value.glm = false;
  }
}

async function testDeepseekConnection() {
  testing.value.deepseek = true;
  deepseekTestResult.value = "正在测试...";
  try {
    const result = await invoke("test_api_connection", { provider: "deepseek" }) as string;
    deepseekTestResult.value = result;
  } catch (err: any) {
    deepseekTestResult.value = typeof err === 'string' ? err : (err.message || JSON.stringify(err));
  } finally {
    testing.value.deepseek = false;
  }
}

async function testAnthropicConnection() {
  testing.value.anthropic = true;
  anthropicTestResult.value = "正在测试...";
  try {
    const result = await invoke("test_api_connection", { provider: "anthropic" }) as string;
    anthropicTestResult.value = result;
  } catch (err: any) {
    anthropicTestResult.value = typeof err === 'string' ? err : (err.message || JSON.stringify(err));
  } finally {
    testing.value.anthropic = false;
  }
}

async function testOpenaiConnection() {
  testing.value.openai = true;
  openaiTestResult.value = "正在测试...";
  try {
    const result = await invoke("test_api_connection", { provider: "openai" }) as string;
    openaiTestResult.value = result;
  } catch (err: any) {
    openaiTestResult.value = typeof err === 'string' ? err : (err.message || JSON.stringify(err));
  } finally {
    testing.value.openai = false;
  }
}

async function testGeminiConnection() {
  testing.value.gemini = true;
  geminiTestResult.value = "正在测试...";
  try {
    const result = await invoke("test_api_connection", { provider: "gemini" }) as string;
    geminiTestResult.value = result;
  } catch (err: any) {
    geminiTestResult.value = typeof err === 'string' ? err : (err.message || JSON.stringify(err));
  } finally {
    testing.value.gemini = false;
  }
}

function triggerOpenaiBenchmarkFileSelect() {
  openaiBenchmarkFileInput.value?.click();
}

function handleOpenaiBenchmarkFileSelect(event: Event) {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0] ?? null;
  openaiBenchmarkFile.value = file;
  openaiBenchmarkFileName.value = file ? file.name : "未选择文件";
  openaiBenchmarkResult.value = null;
  openaiBenchmarkError.value = "";
}

async function runOpenaiBenchmark() {
  if (!openaiStatus.value.configured) {
    alert("请先配置 OpenAI API Key");
    return;
  }
  if (!openaiBenchmarkFile.value) {
    alert("请先选择一篇文章（.docx/.pdf/.txt）");
    return;
  }

  openaiBenchmarkRunning.value = true;
  openaiBenchmarkResult.value = null;
  openaiBenchmarkError.value = "正在并发测试（10并发）...";

  try {
    const file = openaiBenchmarkFile.value;
    const arrayBuffer = await file.arrayBuffer();
    const fileData = Array.from(new Uint8Array(arrayBuffer));

    const result = await invoke("benchmark_gpt_concurrency", {
      fileName: file.name,
      fileData,
      concurrency: 10,
    }) as GptConcurrencyBenchmarkResult;

    openaiBenchmarkResult.value = result;
    openaiBenchmarkError.value = "";
  } catch (err: any) {
    openaiBenchmarkError.value = typeof err === "string" ? err : (err.message || JSON.stringify(err));
  } finally {
    openaiBenchmarkRunning.value = false;
  }
}

</script>

<template>
  <div class="modal-overlay" v-if="visible" @click.self="handleClose">
    <div class="modal-content">
      <div class="modal-header">
        <h2>设置</h2>
        <button class="modal-close" @click="handleClose">&times;</button>
      </div>
      <div class="modal-body">
        <div class="settings-section">
          <h3>API 密钥配置</h3>
          
          <!-- GLM API Key -->
          <div class="form-field">
            <label class="form-label">GLM API Key</label>
            <div class="key-status" v-if="!loading">
              <span v-if="glmStatus.configured" class="status-badge status-ok">
                ✓ 已配置: {{ glmStatus.preview }}
              </span>
              <span v-else class="status-badge status-missing">
                ✗ 未配置
              </span>
              <button v-if="glmStatus.configured" class="btn-delete" @click="deleteGlmKey">删除</button>
              <button v-if="glmStatus.configured" class="btn-test" @click="testGlmConnection" :disabled="testing.glm">
                {{ testing.glm ? '测试中...' : '测试连接' }}
              </button>
            </div>
            <div v-if="glmTestResult" class="test-result" :class="glmTestResult.includes('✓') ? 'test-success' : 'test-error'">
              {{ glmTestResult }}
            </div>
            <div class="input-inline">
              <input class="input-control" placeholder="输入新的 API Key..." v-model="glmKey" type="password" />
              <button class="btn-secondary" type="button" @click="saveGlmKey">保存</button>
            </div>
          </div>
          
          <!-- DeepSeek API Key -->
          <div class="form-field">
            <label class="form-label">DeepSeek API Key</label>
            <div class="key-status" v-if="!loading">
              <span v-if="deepseekStatus.configured" class="status-badge status-ok">
                ✓ 已配置: {{ deepseekStatus.preview }}
              </span>
              <span v-else class="status-badge status-missing">
                ✗ 未配置
              </span>
              <button v-if="deepseekStatus.configured" class="btn-delete" @click="deleteDeepseekKey">删除</button>
              <button v-if="deepseekStatus.configured" class="btn-test" @click="testDeepseekConnection" :disabled="testing.deepseek">
                {{ testing.deepseek ? '测试中...' : '测试连接' }}
              </button>
            </div>
            <div v-if="deepseekTestResult" class="test-result" :class="deepseekTestResult.includes('✓') ? 'test-success' : 'test-error'">
              {{ deepseekTestResult }}
            </div>
            <div class="input-inline">
              <input class="input-control" placeholder="输入新的 API Key..." v-model="deepseekKey" type="password" />
              <button class="btn-secondary" type="button" @click="saveDeepseekKey">保存</button>
            </div>
          </div>

          <!-- Claude (Anthropic) API Key -->
          <div class="form-field">
            <label class="form-label">Claude API Key</label>
            <div class="key-status" v-if="!loading">
              <span v-if="anthropicStatus.configured" class="status-badge status-ok">
                ✓ 已配置: {{ anthropicStatus.preview }}
              </span>
              <span v-else class="status-badge status-missing">
                ✗ 未配置
              </span>
              <button v-if="anthropicStatus.configured" class="btn-delete" @click="deleteAnthropicKey">删除</button>
              <button v-if="anthropicStatus.configured" class="btn-test" @click="testAnthropicConnection" :disabled="testing.anthropic">
                {{ testing.anthropic ? '测试中...' : '测试连接' }}
              </button>
            </div>
            <div v-if="anthropicTestResult" class="test-result" :class="anthropicTestResult.includes('✓') ? 'test-success' : 'test-error'">
              {{ anthropicTestResult }}
            </div>
            <div class="input-inline">
              <input class="input-control" placeholder="输入新的 API Key..." v-model="anthropicKey" type="password" />
              <button class="btn-secondary" type="button" @click="saveAnthropicKey">保存</button>
            </div>
          </div>

          <!-- OpenAI API Key -->
          <div class="form-field">
            <label class="form-label">OpenAI API Key</label>
            <div class="key-status" v-if="!loading">
              <span v-if="openaiStatus.configured" class="status-badge status-ok">
                ✓ 已配置: {{ openaiStatus.preview }}
              </span>
              <span v-else class="status-badge status-missing">
                ✗ 未配置
              </span>
              <button v-if="openaiStatus.configured" class="btn-delete" @click="deleteOpenaiKey">删除</button>
              <button v-if="openaiStatus.configured" class="btn-test" @click="testOpenaiConnection" :disabled="testing.openai">
                {{ testing.openai ? '测试中...' : '测试连接' }}
              </button>
            </div>
            <div v-if="openaiTestResult" class="test-result" :class="openaiTestResult.includes('✓') ? 'test-success' : 'test-error'">
              {{ openaiTestResult }}
            </div>
            <div class="input-inline">
              <input class="input-control" placeholder="输入新的 API Key..." v-model="openaiKey" type="password" />
              <button class="btn-secondary" type="button" @click="saveOpenaiKey">保存</button>
            </div>

            <div class="benchmark-box">
              <div class="key-status">
                <input
                  ref="openaiBenchmarkFileInput"
                  class="hidden-file-input"
                  type="file"
                  accept=".docx,.pdf,.txt"
                  @change="handleOpenaiBenchmarkFileSelect"
                />
                <button class="btn-test" type="button" @click="triggerOpenaiBenchmarkFileSelect">
                  选择文章
                </button>
                <span class="benchmark-file">{{ openaiBenchmarkFileName }}</span>
                <button
                  class="btn-test"
                  type="button"
                  @click="runOpenaiBenchmark"
                  :disabled="openaiBenchmarkRunning || !openaiStatus.configured"
                >
                  {{ openaiBenchmarkRunning ? "并发测试中..." : "并发测试（10）" }}
                </button>
              </div>

              <div v-if="openaiBenchmarkError" class="test-result test-error">
                {{ openaiBenchmarkError }}
              </div>

              <div v-if="openaiBenchmarkResult" class="test-result test-success">
                总耗时 {{ openaiBenchmarkResult.totalMs }}ms｜成功 {{ openaiBenchmarkResult.success }}/{{ openaiBenchmarkResult.concurrency }}｜
                min/avg/max {{ openaiBenchmarkResult.minMs ?? '-' }}/{{ openaiBenchmarkResult.avgMs?.toFixed(1) ?? '-' }}/{{ openaiBenchmarkResult.maxMs ?? '-' }}ms
              </div>
            </div>
          </div>

          <!-- Gemini API Key -->
          <div class="form-field">
            <label class="form-label">Gemini API Key</label>
            <div class="key-status" v-if="!loading">
              <span v-if="geminiStatus.configured" class="status-badge status-ok">
                ✓ 已配置: {{ geminiStatus.preview }}
              </span>
              <span v-else class="status-badge status-missing">
                ✗ 未配置
              </span>
              <button v-if="geminiStatus.configured" class="btn-delete" @click="deleteGeminiKey">删除</button>
              <button v-if="geminiStatus.configured" class="btn-test" @click="testGeminiConnection" :disabled="testing.gemini">
                {{ testing.gemini ? '测试中...' : '测试连接' }}
              </button>
            </div>
            <div v-if="geminiTestResult" class="test-result" :class="geminiTestResult.includes('✓') ? 'test-success' : 'test-error'">
              {{ geminiTestResult }}
            </div>
            <div class="input-inline">
              <input class="input-control" placeholder="输入新的 API Key..." v-model="geminiKey" type="password" />
              <button class="btn-secondary" type="button" @click="saveGeminiKey">保存</button>
            </div>
          </div>

          <div class="hint-text">
            提示：双模式检测需要同时配置 GLM 和 DeepSeek 密钥
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 70, 67, 0.7);
  backdrop-filter: blur(3px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--bg-surface);
  border-radius: var(--radius-card);
  width: 90%;
  max-width: 500px;
  max-height: 85vh;
  overflow-y: auto;
  box-shadow: var(--shadow-xl);
  border: 2px solid var(--border-dark);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 2px solid var(--border-dark);
  position: sticky;
  top: 0;
  background: var(--bg-surface);
  z-index: 1;
}

.modal-header h2 {
  margin: 0;
  font-size: var(--font-lg);
  color: var(--text-dark);
}

.modal-close {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  font-size: 24px;
  cursor: pointer;
  color: var(--text-dark);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.modal-close:hover {
  background: rgba(225, 97, 98, 0.1);
  color: var(--accent);
}

.modal-body {
  padding: 20px;
}

.settings-section h3 {
  margin: 0 0 16px 0;
  font-size: var(--font-base);
  color: var(--text-dark);
}

.form-field {
  margin-bottom: 16px;
}

.form-label {
  display: block;
  margin-bottom: 6px;
  font-size: var(--font-sm);
  color: var(--text-surface-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-weight: 600;
}

.input-inline {
  display: flex;
  gap: 8px;
}

.input-control {
  flex: 1;
  padding: 10px 12px;
  border: 2px solid var(--border-dark);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  background: var(--bg-input);
  color: var(--text-dark);
  box-shadow: var(--shadow-sm);
  transition: all var(--transition-fast);
}

.input-control:hover {
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-md);
}

.input-control:focus {
  outline: none;
  border-color: var(--primary);
}

.btn-secondary {
  padding: 10px 16px;
  background: var(--secondary);
  border: 2px solid var(--border-dark);
  border-radius: var(--radius-sm);
  font-weight: 600;
  cursor: pointer;
  color: var(--text-dark);
  box-shadow: var(--shadow-sm);
  transition: all var(--transition-fast);
}

.btn-secondary:hover {
  background: var(--primary);
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-md);
}

.key-status {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.benchmark-box {
  margin-top: 10px;
}

.hidden-file-input {
  display: none;
}

.benchmark-file {
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-sm);
  color: var(--text-dark);
  opacity: 0.8;
}

.status-badge {
  font-size: var(--font-sm);
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  font-family: monospace;
}

.status-ok {
  background: rgba(76, 175, 80, 0.2);
  color: #4CAF50;
  border: 1px solid #4CAF50;
}

.status-missing {
  background: rgba(225, 97, 98, 0.2);
  color: var(--accent);
  border: 1px solid var(--accent);
}

.btn-delete {
  padding: 4px 8px;
  font-size: 12px;
  background: transparent;
  border: 1px solid var(--accent);
  color: var(--accent);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-delete:hover {
  background: rgba(225, 97, 98, 0.2);
}

.hint-text {
  margin-top: 16px;
  padding: 12px;
  background: rgba(249, 188, 96, 0.15);
  border: 1px solid var(--primary);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  color: #b8860b;
}

.btn-test {
  padding: 4px 10px;
  font-size: 12px;
  background: rgba(171, 209, 198, 0.3);
  border: 1px solid var(--secondary);
  color: var(--text-dark);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-test:hover {
  background: var(--secondary);
}

.btn-test:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.test-result {
  padding: 8px 12px;
  margin: 8px 0;
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  font-family: monospace;
}

.test-success {
  background: rgba(76, 175, 80, 0.1);
  border: 1px solid #4CAF50;
  color: #4CAF50;
}

.test-error {
  background: rgba(225, 97, 98, 0.1);
  border: 1px solid var(--accent);
  color: var(--accent);
}
</style>
