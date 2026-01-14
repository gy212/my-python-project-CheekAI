<script setup lang="ts">
import { ref, onMounted } from "vue";
import "@/styles/variables.css";

// Components
import TitleBar from "@/components/TitleBar.vue";
import LoadingMask from "@/components/LoadingMask.vue";
import SettingsModal from "@/components/SettingsModal.vue";
import ControlPanel from "@/components/ControlPanel.vue";
import TextInput from "@/components/TextInput.vue";
import ResultsPanel from "@/components/ResultsPanel.vue";

// Composables
import { useDetection, useProviders, useFileHandler } from "@/composables";

// Initialize composables
const {
  inputText,
  sensitivity,
  selectedProvider,
  dualMode,
  isLoading,
  loadingText,
  progress,
  segments,
  aggregation,
  dualResult,
  filterSummary,
  hasResult,
  overallDecision,
  overallProbability,
  detect,
  exportJson,
  exportCsv,
} = useDetection();

const { providerOptions, fetchProviders, saveApiKey } = useProviders();
const { fileName, fileInput, triggerFileSelect, handleFileSelect } = useFileHandler();

// UI State
const settingsOpen = ref(false);

// Event handlers
function onFileSelect(event: Event) {
  handleFileSelect(event, (text: string) => {
    inputText.value = text;
  });
}

async function onSaveApiKey(provider: string, key: string) {
  await saveApiKey(provider, key);
}

// Lifecycle
onMounted(() => {
  fetchProviders();
});
</script>

<template>
  <!-- Title Bar -->
  <TitleBar />
  
  <div class="app-shell">
    <div class="main-area">
      <main class="content-grid">
        <!-- Left Column: Controls + Input -->
        <section class="column column-controls">
          <ControlPanel
            :sensitivity="sensitivity"
            :selectedProvider="selectedProvider"
            :providerOptions="providerOptions"
            :fileName="fileName"
            :dualMode="dualMode"
            @update:sensitivity="sensitivity = $event"
            @update:selectedProvider="selectedProvider = $event"
            @update:dualMode="dualMode = $event"
            @detect="detect"
            @open-settings="settingsOpen = true"
            @trigger-file-select="triggerFileSelect"
          />
          
          <TextInput v-model="inputText" />
          
          <!-- Hidden file input -->
          <input 
            type="file" 
            class="file-input-hidden" 
            ref="fileInput" 
            @change="onFileSelect" 
          />
        </section>
        
        <!-- Right Column: Results -->
        <ResultsPanel
          :segments="segments"
          :aggregation="aggregation"
          :dualResult="dualResult"
          :hasResult="hasResult"
          :overallProbability="overallProbability"
          :overallDecision="overallDecision"
          :originalText="inputText"
          :filterSummary="filterSummary"
          @export-json="exportJson"
          @export-csv="exportCsv"
        />
      </main>
    </div>
  </div>
  
  <!-- Loading Mask -->
  <LoadingMask :visible="isLoading" :text="loadingText" :progress="progress" />
  
  <!-- Settings Modal -->
  <SettingsModal
    :visible="settingsOpen"
    @close="settingsOpen = false"
    @save-key="onSaveApiKey"
  />
</template>

<style>
/* App Layout Styles */
.app-shell {
  min-height: 100vh;
  padding-top: 32px;
}

.main-area {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.content-grid {
  flex: 1;
  display: grid;
  grid-template-columns: 380px 1fr;
  gap: 32px;
  padding: 32px 32px 48px;
  align-items: start;
}

.column {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.file-input-hidden {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

@media (max-width: 1280px) {
  .content-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .content-grid {
    padding: 24px;
  }
}
</style>
