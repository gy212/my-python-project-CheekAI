// Providers Composable
// Handles API provider and API key management

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderInfo, ProviderOption } from "@/types";

export function useProviders() {
  // State
  const providerOptions = ref<ProviderOption[]>([]);
  const glmKey = ref("");
  const deepseekKey = ref("");

  // Computed
  function getCurrentProviderLabel(selectedProvider: string): string {
    if (!selectedProvider) return "任意 LLM 判别";
    const found = providerOptions.value.find(o => o.value === selectedProvider);
    return found ? found.label : selectedProvider;
  }

  // Methods
  async function fetchProviders() {
    try {
      console.log("[fetchProviders] Fetching providers...");
      const res = await invoke("get_providers");
      console.log("[fetchProviders] Response:", res);
      const data = res as ProviderInfo[];
      const list: ProviderOption[] = [];
      
      if (Array.isArray(data)) {
        for (const provider of data) {
          // Each provider can have multiple models
          const models = provider.name === "glm" 
            ? ["glm-4-plus", "glm-4.6", "glm-4-flash"] 
            : ["deepseek-chat", "deepseek-reasoner"];
          
          for (const model of models) {
            list.push({
              value: `${provider.name}:${model}`,
              label: `${provider.display_name} - ${model}${provider.has_key ? '' : ' (未配置密钥)'}`
            });
          }
        }
      }
      console.log("[fetchProviders] Provider options:", list);
      providerOptions.value = list;
    } catch (err) {
      console.error("[fetchProviders] Error:", err);
    }
  }

  async function saveApiKey(provider: string, key: string) {
    if (!key.trim()) {
      alert("请输入 API Key");
      return;
    }

    try {
      console.log(`[saveApiKey] Saving ${provider} API key...`);
      await invoke("store_api_key", {
        provider: provider,
        key: key.trim(),
      });
      console.log(`[saveApiKey] ${provider} API key saved successfully`);
      alert("API Key 已保存");
      
      // Clear input field
      if (provider === "glm") {
        glmKey.value = "";
      } else {
        deepseekKey.value = "";
      }
      
      // Refresh providers
      console.log("[saveApiKey] Refreshing providers...");
      await fetchProviders();
      console.log("[saveApiKey] Providers refreshed");
    } catch (err: any) {
      console.error("[saveApiKey] Error:", err);
      alert("保存失败: " + (typeof err === 'string' ? err : (err.message || JSON.stringify(err))));
    }
  }

  return {
    // State
    providerOptions,
    glmKey,
    deepseekKey,
    // Methods
    getCurrentProviderLabel,
    fetchProviders,
    saveApiKey,
  };
}
