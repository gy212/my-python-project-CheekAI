// Detection Composable
// Handles AI text detection logic

import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  SegmentResponse,
  AggregationResponse,
  DualDetectionResult,
  DetectTextRequest,
  FilterSummary,
  DocumentProfile
} from "@/types";

interface ProgressEvent {
  stage: string;
  progress: number;
  current?: number;
  total?: number;
}

export function useDetection() {
  // State
  const inputText = ref("");
  const sensitivity = ref("medium");
  const selectedProvider = ref("");
  const dualMode = ref(false);
  const isLoading = ref(false);
  const loadingText = ref("正在检测...");
  const progress = ref(0);
  const segments = ref<SegmentResponse[]>([]);
  const aggregation = ref<AggregationResponse | null>(null);
  const dualResult = ref<DualDetectionResult | null>(null);
  const filterSummary = ref<FilterSummary | undefined>(undefined);
  const documentProfile = ref<DocumentProfile | null>(null);

  // Computed
  const hasResult = computed(() => segments.value.length > 0);
  const overallDecision = computed(() => aggregation.value?.decision || "");
  const overallProbability = computed(() =>
    aggregation.value ? (aggregation.value.overallProbability * 100).toFixed(1) : "0"
  );

  // Methods
  function getStageText(event: ProgressEvent): string {
    switch (event.stage) {
      case "preprocessing": return "正在预处理文本...";
      case "building_blocks": return `正在构建段落块 (${event.total || 0} 个)...`;
      case "analyzing": return event.total
        ? `正在分析段落 (${event.current || 0}/${event.total})...`
        : "正在分析文本...";
      case "analyzing_dual": return "正在进行双模式分析...";
      case "aggregating": return "正在汇总结果...";
      case "complete": return "检测完成";
      default: return "正在检测...";
    }
  }

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
    progress.value = 0;

    let unlisten: UnlistenFn | null = null;
    try {
      // Listen for progress events
      unlisten = await listen<ProgressEvent>("detection-progress", (event) => {
        progress.value = event.payload.progress;
        loadingText.value = getStageText(event.payload);
      });

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
        filterSummary.value = data.filterSummary;
        documentProfile.value = data.documentProfile ?? null;
      } else {
        segments.value = data.segments || [];
        aggregation.value = data.aggregation;
        dualResult.value = null;
        filterSummary.value = data.filterSummary;
        documentProfile.value = data.documentProfile ?? null;
      }
    } catch (err: any) {
      alert("检测失败: " + (err.message || err));
    } finally {
      if (unlisten) unlisten();
      isLoading.value = false;
      progress.value = 0;
    }
  }

  function clearResults() {
    segments.value = [];
    aggregation.value = null;
    dualResult.value = null;
    filterSummary.value = undefined;
    documentProfile.value = null;
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
    const rows = [["段落ID", "原始概率", "置信度", "不确定度", "决策"]];
    segments.value.forEach((seg: any) => {
      const rawProb = seg.rawProbability ?? seg.aiProbability ?? 0;
      const uncertainty = seg.uncertainty ?? 0;
      rows.push([
        seg.chunkId,
        (rawProb * 100).toFixed(1) + "%",
        (seg.confidence * 100).toFixed(1) + "%",
        (uncertainty * 100).toFixed(1) + "%",
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
    progress,
    segments,
    aggregation,
    dualResult,
    filterSummary,
    documentProfile,
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
