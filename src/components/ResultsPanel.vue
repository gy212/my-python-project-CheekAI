<script setup lang="ts">
import type { SegmentResponse, AggregationResponse, DualDetectionResult } from "@/types";

defineProps<{
  segments: SegmentResponse[];
  aggregation: AggregationResponse | null;
  dualResult: DualDetectionResult | null;
  hasResult: boolean;
  overallProbability: string;
  overallDecision: string;
}>();

defineEmits<{
  (e: "export-json"): void;
  (e: "export-csv"): void;
}>();

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
  if (prob <= 0.30) return "prob-low";      // ≤30% 绿色
  if (prob < 0.70) return "prob-medium";    // 30-70% 黄色
  return "prob-high";                        // ≥70% 红色
}
</script>

<template>
  <section class="column column-results">
    <!-- Segments Card -->
    <article class="surface card segments-card">
      <header class="card-heading">
        <div>
          <p class="eyebrow">段落列表</p>
          <h2>分段评分</h2>
        </div>
        <div class="card-actions" v-if="hasResult">
          <button id="exportJson" class="btn-link" type="button" @click="$emit('export-json')">导出 JSON</button>
          <button id="exportCsv" class="btn-link" type="button" @click="$emit('export-csv')">导出 CSV</button>
        </div>
      </header>
      
      <!-- Overall Result Summary -->
      <div v-if="aggregation" class="batch-summary" style="margin-bottom: 20px;">
        <div class="metrics">
          <div class="metric">
            <span class="metric-label">整体概率</span>
            <span class="metric-value" :class="getProbabilityClass(aggregation.overallProbability)">{{ overallProbability }}%</span>
          </div>
          <div class="metric">
            <span class="metric-label">决策</span>
            <span class="metric-value" :class="getDecisionClass(overallDecision)">{{ getDecisionText(overallDecision) }}</span>
          </div>
          <div class="metric">
            <span class="metric-label">置信度</span>
            <span class="metric-value">{{ (aggregation.overallConfidence * 100).toFixed(1) }}%</span>
          </div>
        </div>
      </div>

      <div id="segments" class="segments-grid">
        <div 
          v-for="seg in segments" 
          :key="seg.chunkId" 
          class="segment-card node" 
          :class="getProbabilityClass(seg.aiProbability)"
        >
          <div class="segment-header">段落 {{ seg.chunkId + 1 }}</div>
          <div class="row" style="margin-top:8px; justify-content: space-between;">
            <span>概率: {{ (seg.aiProbability * 100).toFixed(1) }}%</span>
            <span>置信度: {{ (seg.confidence * 100).toFixed(1) }}%</span>
          </div>
        </div>
        <div v-if="!hasResult" style="grid-column: 1 / -1; text-align: center; padding: 20px; color: var(--text-muted);">
          暂无检测结果
        </div>
      </div>
    </article>

    <!-- Dual Mode Comparison Card -->
    <article class="surface card batch-card" v-if="dualResult">
      <header class="card-heading">
        <div>
          <p class="eyebrow">双模式对比</p>
          <h2>检测结果</h2>
        </div>
      </header>
      <div class="metrics">
        <div class="metric">
          <span class="metric-label">段落模式</span>
          <span class="metric-value">{{ (dualResult.paragraph?.aggregation.overallProbability * 100).toFixed(1) }}%</span>
        </div>
        <div class="metric">
          <span class="metric-label">句子模式</span>
          <span class="metric-value">{{ (dualResult.sentence?.aggregation.overallProbability * 100).toFixed(1) }}%</span>
        </div>
        <div class="metric">
          <span class="metric-label">一致性</span>
          <span class="metric-value">{{ (dualResult.comparison?.consistencyScore * 100).toFixed(1) }}%</span>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.column {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.card {
  padding: 24px;
}

.card-heading {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
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

.card-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.segments-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
  min-height: 160px;
}

.batch-summary {
  font-size: var(--font-sm);
  color: var(--text-surface-muted);
  margin-bottom: 12px;
}

.node {
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  background: var(--secondary);
  color: var(--text-dark);
  font-size: var(--font-sm);
  border: 2px solid var(--border-dark);
  box-shadow: var(--shadow-sm);
  transition: all var(--transition-fast);
}

.node:hover {
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-md);
}

.metrics {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 16px;
}

.metric {
  border-radius: var(--radius-sm);
  padding: 12px 14px;
  border: 2px solid var(--border-dark);
  background: var(--bg-input);
  display: flex;
  justify-content: space-between;
  align-items: center;
  box-shadow: var(--shadow-sm);
  transition: all var(--transition-fast);
}

.metric:hover {
  transform: translate(-1px, -1px);
  box-shadow: var(--shadow-md);
}

.metric-label {
  font-size: var(--font-xs);
  color: var(--text-dark);
  font-weight: 600;
  text-transform: uppercase;
}

.metric-value {
  font-weight: 700;
  color: var(--text-dark);
}

.row {
  display: flex;
  gap: 12px;
}

.segment-header {
  font-size: var(--font-base);
  font-weight: 800;
  color: var(--text-dark);
  background: rgba(255, 255, 255, 0.6);
  padding: 4px 10px;
  border-radius: var(--radius-sm);
  display: inline-block;
  box-shadow: 1px 1px 0px rgba(0,0,0,0.1);
  border: 1px solid rgba(0,0,0,0.1);
}

.segment-card {
  cursor: pointer;
  transition: all var(--transition-fast);
}

.segment-card:hover {
  transform: translate(-2px, -2px);
  box-shadow: var(--shadow-lg);
}

.segment-card:active {
  transform: translate(1px, 1px);
  box-shadow: 1px 1px 0px var(--border-dark);
}

/* Decision classes */
.decision-pass {
  background: #d1fae5;
  color: #065f46;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  font-weight: 600;
  border: 1px solid #065f46;
}

.decision-review {
  background: #fef3c7;
  color: #92400e;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  font-weight: 600;
  border: 1px solid #92400e;
}

.decision-flag {
  background: #fee2e2;
  color: #991b1b;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  font-weight: 600;
  border: 1px solid #991b1b;
}

/* Probability colors */
.prob-low {
  border-left: 4px solid var(--success);
  background: rgba(34, 197, 94, 0.15);
}

.prob-medium {
  border-left: 4px solid var(--warning);
  background: rgba(234, 179, 8, 0.15);
}

.prob-high {
  border-left: 4px solid var(--danger);
  background: rgba(225, 97, 98, 0.15);
}

.btn-link {
  height: 40px;
  border-radius: var(--radius-pill);
  border: 2px solid transparent;
  cursor: pointer;
  font-size: var(--font-sm);
  font-weight: 700;
  letter-spacing: 0.5px;
  transition: all var(--transition-fast);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--accent);
  padding: 0 12px;
}

.btn-link:hover {
  background: rgba(225, 97, 98, 0.1);
  border-color: var(--accent);
}

.btn-link:focus-visible {
  outline: 3px solid var(--primary);
  outline-offset: 2px;
}
</style>
