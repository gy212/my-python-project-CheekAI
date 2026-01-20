// CheekAI Data Models
// Migrated from Python Pydantic schemas

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export FilterSummary from detection module
pub use crate::services::detection::content_filter::{FilterSummary, ParagraphCategory, ParagraphClassification};

// ============ Preprocess & Chunking Options ============

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreprocessOptions {
    #[serde(default)]
    pub strip_html: bool,
    #[serde(default)]
    pub redact_pii: bool,
    #[serde(default = "default_true")]
    pub normalize_punctuation: bool,
    #[serde(default = "default_true")]
    pub auto_language: bool,
    #[serde(default = "default_chunk_size")]
    pub chunk_size_tokens: i32,
    #[serde(default = "default_overlap")]
    pub overlap_tokens: i32,
    #[serde(default = "default_true")]
    pub align_to_paragraphs: bool,
    #[serde(default = "default_merge_min")]
    pub paragraph_merge_min_chars: i32,
    #[serde(default = "default_split_max")]
    pub paragraph_split_max_sentence_len: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkingOptions {
    #[serde(default = "default_chunk_size")]
    pub chunk_size_tokens: i32,
    #[serde(default = "default_overlap")]
    pub overlap_tokens: i32,
}

impl Default for ChunkingOptions {
    fn default() -> Self {
        Self {
            chunk_size_tokens: 1500,
            overlap_tokens: 150,
        }
    }
}

// ============ Detection Request ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectRequest {
    pub text: String,
    pub language: Option<String>,
    pub genre: Option<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default = "default_true")]
    pub use_perplexity: bool,
    #[serde(default = "default_true")]
    pub use_stylometry: bool,
    #[serde(default)]
    pub preprocess_options: PreprocessOptions,
    #[serde(default)]
    pub chunking: ChunkingOptions,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,
}

// ============ Signal Types ============

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SignalStylometry {
    pub ttr: f64,
    pub avg_sentence_len: f64,
    pub function_word_ratio: Option<f64>,
    pub repeat_ratio: Option<f64>,
    pub punctuation_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalPerplexity {
    pub ppl: Option<f64>,
    pub z: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalLLMJudgment {
    pub prob: Option<f64>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub uncertainty: Option<f64>,
    #[serde(default)]
    pub evidence: Vec<SignalLLMEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SignalLLMEvidence {
    pub id: String,
    pub score: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSignals {
    pub llm_judgment: SignalLLMJudgment,
    pub perplexity: SignalPerplexity,
    pub stylometry: SignalStylometry,
}

// ============ Segment Response ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentOffsets {
    /// UTF-8 byte offset (0-based) into the analyzed text.
    pub start: i32,
    /// UTF-8 byte offset (0-based, end-exclusive) into the analyzed text.
    pub end: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentResponse {
    pub chunk_id: i32,
    pub language: String,
    pub offsets: SegmentOffsets,
    #[serde(alias = "aiProbability")]
    pub raw_probability: f64,
    pub confidence: f64,
    #[serde(default)]
    pub uncertainty: f64,
    #[serde(default)]
    pub decision: String,
    pub signals: SegmentSignals,
    pub explanations: Vec<String>,
}

// ============ Aggregation ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionThresholds {
    pub review: f64,
    pub flag: f64,
}

impl Default for DecisionThresholds {
    fn default() -> Self {
        Self {
            review: 0.65,
            flag: 0.85,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationThresholds {
    #[serde(default = "default_low")]
    pub low: f64,
    #[serde(default = "default_medium")]
    pub medium: f64,
    #[serde(default = "default_high")]
    pub high: f64,
    #[serde(default = "default_very_high")]
    pub very_high: f64,
}

impl Default for AggregationThresholds {
    fn default() -> Self {
        Self {
            low: 0.65,
            medium: 0.75,
            high: 0.85,
            very_high: 0.90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationResponse {
    pub overall_probability: f64,
    pub overall_confidence: f64,
    pub method: String,
    pub thresholds: AggregationThresholds,
    #[serde(default)]
    pub decision_thresholds: DecisionThresholds,
    pub rubric_version: String,
    pub decision: String,
    pub buffer_margin: f64,
    pub stylometry_probability: Option<f64>,
    pub quality_score_normalized: Option<f64>,
    pub block_weights: Option<HashMap<String, f64>>,
    pub dimension_scores: Option<HashMap<String, i32>>,
}

// ============ Dual Detection ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeDetectionResult {
    pub aggregation: AggregationResponse,
    pub segments: Vec<SegmentResponse>,
    pub segment_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivergentRegion {
    pub paragraph_segment_id: i32,
    pub sentence_segment_id: i32,
    pub probability_diff: f64,
    pub paragraph_prob: f64,
    pub sentence_prob: f64,
    pub text_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonResult {
    pub probability_diff: f64,
    pub consistency_score: f64,
    #[serde(default)]
    pub divergent_regions: Vec<DivergentRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DualDetectionResult {
    pub paragraph: ModeDetectionResult,
    pub sentence: ModeDetectionResult,
    pub comparison: ComparisonResult,
    /// Fused aggregation combining paragraph and sentence results
    /// Weight: paragraph 0.6 + sentence 0.4
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fused_aggregation: Option<AggregationResponse>,
    /// Optional filter summary (titles/TOC/references/etc.) for transparency in dual-mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_summary: Option<FilterSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_profile: Option<DocumentProfile>,
}

// ============ Cost & Preprocess Summary ============

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreprocessSummary {
    pub language: String,
    pub chunks: i32,
    #[serde(default)]
    pub redacted: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CostBreakdown {
    pub tokens: i32,
    pub latency_ms: i32,
    #[serde(default)]
    pub provider_breakdown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentProfile {
    pub category: String,
    pub summary: String,
    #[serde(default, alias = "field")]
    pub discipline: Option<String>,
    #[serde(default)]
    pub subfield: Option<String>,
    #[serde(default, alias = "paperType", alias = "paper_type")]
    pub paper_type: Option<String>,
    #[serde(default)]
    pub conventions: Vec<String>,
    #[serde(default)]
    pub validity: String,
}

// ============ Detection Response ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectResponse {
    pub aggregation: AggregationResponse,
    pub segments: Vec<SegmentResponse>,
    pub preprocess_summary: PreprocessSummary,
    pub cost: CostBreakdown,
    pub version: String,
    pub request_id: String,
    pub dual_detection: Option<DualDetectionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_profile: Option<DocumentProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_summary: Option<FilterSummary>,
}

// ============ Batch Detection ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemRequest {
    pub id: String,
    pub text: String,
    pub language: Option<String>,
    pub genre: Option<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default = "default_true")]
    pub use_perplexity: bool,
    #[serde(default = "default_true")]
    pub use_stylometry: bool,
    #[serde(default)]
    pub preprocess_options: PreprocessOptions,
    #[serde(default)]
    pub chunking: ChunkingOptions,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemResponse {
    pub id: String,
    pub aggregation: AggregationResponse,
    pub segments: Vec<SegmentResponse>,
    pub preprocess_summary: PreprocessSummary,
    pub cost: CostBreakdown,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub count: i32,
    pub fail_count: i32,
    pub avg_probability: f64,
    pub p95_probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDetectRequest {
    pub items: Vec<BatchItemRequest>,
    #[serde(default = "default_parallel")]
    pub parallel: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDetectResponse {
    pub items: Vec<BatchItemResponse>,
    pub summary: BatchSummary,
}

// ============ Paper Analysis ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperAnalyzeRequest {
    pub text: String,
    pub language: Option<String>,
    pub genre: Option<String>,
    #[serde(default = "default_rounds")]
    pub rounds: i32,
    #[serde(default = "default_true")]
    pub use_llm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadabilityScores {
    pub fluency: f64,
    pub clarity: f64,
    pub cohesion: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionItem {
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiRoundDetail {
    pub round: i32,
    pub probability: f64,
    pub confidence: f64,
    pub template_id: Option<String>,
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiRoundSummary {
    pub rounds: i32,
    pub avg_probability: f64,
    pub avg_confidence: f64,
    pub variance: f64,
    pub details: Vec<MultiRoundDetail>,
    pub trimmed_avg_probability: Option<f64>,
    pub trimmed_avg_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperAnalyzeResponse {
    pub aggregation: AggregationResponse,
    pub readability: ReadabilityScores,
    pub multi_round: MultiRoundSummary,
    pub suggestions: Vec<SuggestionItem>,
}

// ============ History ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub ts: String,
    pub req_params: HashMap<String, serde_json::Value>,
    pub aggregation: AggregationResponse,
    pub multi_round: Option<MultiRoundSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySaveRequest {
    pub id: String,
    pub req_params: HashMap<String, serde_json::Value>,
    pub aggregation: AggregationResponse,
    pub multi_round: Option<MultiRoundSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySaveResponse {
    pub ok: bool,
    pub total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryListResponse {
    pub items: Vec<HistoryItem>,
}

// ============ Review ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewLogItem {
    pub ts: String,
    pub request_id: String,
    pub overall_probability: f64,
    pub overall_confidence: f64,
    pub decision: String,
    pub label: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSubmitRequest {
    pub request_id: String,
    pub decision: String,
    pub overall_probability: f64,
    pub overall_confidence: f64,
    pub label: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSubmitResponse {
    pub ok: bool,
    pub total: i32,
    pub pass_count: i32,
    pub review_count: i32,
    pub flag_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSummaryResponse {
    pub total: i32,
    pub labeled: i32,
    pub tp: i32,
    pub tn: i32,
    pub fp: i32,
    pub r#fn: i32,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

// ============ Consistency Check ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheckRequest {
    pub segments: Vec<SegmentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyIssue {
    pub segment_id: i32,
    pub r#type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheckResponse {
    pub ok: bool,
    pub issues: Vec<ConsistencyIssue>,
}

// ============ Calibration ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrateItem {
    pub prob: f64,
    pub label: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrateRequest {
    pub items: Vec<CalibrateItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct CalibrateResponse {
    pub ok: bool,
    pub version: String,
    #[serde(default)]
    pub a: f64,
    #[serde(default)]
    pub b: f64,
}

// ============ Preprocess Upload ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreprocessUploadResponse {
    pub normalized_text: String,
    pub preprocess_summary: PreprocessSummary,
    pub segments: Vec<SegmentResponse>,
    #[serde(default)]
    pub structured_nodes: Vec<HashMap<String, serde_json::Value>>,
    pub formatted_text: Option<String>,
    pub format_summary: Option<HashMap<String, serde_json::Value>>,
    pub mapping: Option<HashMap<String, serde_json::Value>>,
    pub comparison: Option<HashMap<String, serde_json::Value>>,
}

// ============ Prompt Variants ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptVariant {
    pub id: String,
    pub name: String,
    pub style: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariantsResponse {
    pub items: Vec<PromptVariant>,
}

// ============ Default Value Functions ============

fn default_true() -> bool { true }
fn default_chunk_size() -> i32 { 1500 }
fn default_overlap() -> i32 { 150 }
fn default_merge_min() -> i32 { 200 }
fn default_split_max() -> i32 { 120 }
fn default_sensitivity() -> String { "medium".to_string() }
fn default_parallel() -> Option<i32> { Some(4) }
fn default_rounds() -> i32 { 6 }
fn default_low() -> f64 { 0.65 }
fn default_medium() -> f64 { 0.75 }
fn default_high() -> f64 { 0.85 }
fn default_very_high() -> f64 { 0.90 }
