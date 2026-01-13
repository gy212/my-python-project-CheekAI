// Detection Composable
// Handles AI text detection logic

import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { 
  SegmentResponse, 
  AggregationResponse, 
  DualDetectionResult,
  DetectTextRequest 
} from "@/types";

export function useDetection() {
  // State
  const inputText = ref("");
  const sensitivity = ref("medium");
  const selectedProvider = ref("");
  const dualMode = ref(false);
  const isLoading = ref(false);
  const loadingText = ref("正在检测...");
  const segments = ref<SegmentResponse[]>([]);
  const aggregation = ref<AggregationResponse | null>(null);
  const dualResult = ref<DualDetectionResult | null>(null);

  // Computed
  const hasResult = computed(() => segments.value.length > 0);
  const overallDecision = computed(() => aggregation.value?.decision || "");
  const overallProbability = computed(() =>
    aggregation.value ? (aggregation.value.overallProbability * 100).toFixed(1) : "0"
  );

  // Methods
  async function detect() {
    if (isLoading.value) {
      return;
    }
    if (!inputText.value.trim()) {
      alert("请输入待检测文本");
      return;
    }

    isLoading.value = true;
    loadingText.value = "正在检测...";

    try {
      const cmd = dualMode.value ? "detect_dual_mode" : "detect_text";
      const request: DetectTextRequest = {
        text: inputText.value,
        usePerplexity: true,
        useStylometry: true,
        sensitivity: sensitivity.value,
        provider: selectedProvider.value || null,
        dualMode: dualMode.value,
      };

      const result = await invoke(cmd, { request });
      const data = result as any;

      if (dualMode.value) {
        dualResult.value = data;
        segments.value = data.paragraph?.segments || [];
        aggregation.value = data.paragraph?.aggregation;
      } else {
        segments.value = data.segments || [];
        aggregation.value = data.aggregation;
        dualResult.value = null;
      }
    } catch (err: any) {
      alert("检测失败: " + (err.message || err));
    } finally {
      isLoading.value = false;
    }
  }

  function clearResults() {
    segments.value = [];
    aggregation.value = null;
    dualResult.value = null;
  }

  // UI Helper Functions
  function getDecisionClass(decision: string) {
    switch (decision) {
      case "pass": return "decision-pass";
      case "review": return "decision-review";
      case "flag": return "decision-flag";
      default: return "";
    }
  }

  function getDecisionText(decision: string) {
    switch (decision) {
      case "pass": return "通过";
      case "review": return "待审";
      case "flag": return "标记";
      default: return decision;
    }
  }

  function getProbabilityClass(prob: number) {
    if (prob < 0.65) return "prob-low";
    if (prob < 0.85) return "prob-medium";
    return "prob-high";
  }

  // Export Functions
  function exportJson() {
    const data = {
      aggregation: aggregation.value,
      segments: segments.value,
      dualDetection: dualResult.value,
    };
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "cheekAI_result.json";
    a.click();
    URL.revokeObjectURL(url);
  }

  function exportCsv() {
    const rows = [["段落ID", "AI概率", "置信度", "决策"]];
    segments.value.forEach((seg: any) => {
      rows.push([
        seg.chunkId,
        (seg.aiProbability * 100).toFixed(1) + "%",
        (seg.confidence * 100).toFixed(1) + "%",
        seg.decision || "",
      ]);
    });
    const csv = rows.map(r => r.join(",")).join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "cheekAI_result.csv";
    a.click();
    URL.revokeObjectURL(url);
  }

  return {
    // State
    inputText,
    sensitivity,
    selectedProvider,
    dualMode,
    isLoading,
    loadingText,
    segments,
    aggregation,
    dualResult,
    // Computed
    hasResult,
    overallDecision,
    overallProbability,
    // Methods
    detect,
    clearResults,
    getDecisionClass,
    getDecisionText,
    getProbabilityClass,
    exportJson,
    exportCsv,
  };
}
