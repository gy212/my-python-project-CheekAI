// CheekAI TypeScript Type Definitions

// Provider Types
export interface ProviderInfo {
  name: string;
  display_name: string;
  has_key: boolean;
}

export interface ProviderOption {
  value: string;
  label: string;
}

// Sensitivity Types
export interface SensitivityOption {
  value: string;
  label: string;
}

export const SENSITIVITY_OPTIONS: SensitivityOption[] = [
  { value: "low", label: "低敏感" },
  { value: "medium", label: "中敏感" },
  { value: "high", label: "高敏感" },
];

// Detection Types
export interface SegmentSignals {
  llm_judgment: {
    prob: number | null;
    models: string[];
  };
  perplexity: {
    ppl: number | null;
    z: number | null;
  };
  stylometry: {
    ttr: number;
    avg_sentence_len: number;
    function_word_ratio: number | null;
    repeat_ratio: number | null;
    punctuation_ratio: number | null;
  };
}

export interface SegmentResponse {
  chunkId: number;
  language: string;
  offsets: {
    /** UTF-8 byte offsets from Rust (0-based, end-exclusive). Not JS string indices. */
    start: number;
    end: number;
  };
  aiProbability: number;
  confidence: number;
  signals: SegmentSignals;
  explanations: string[];
}

export interface AggregationResponse {
  overallProbability: number;
  overallConfidence: number;
  method: string;
  thresholds: {
    low: number;
    medium: number;
    high: number;
    veryHigh: number;
  };
  rubricVersion: string;
  decision: string;
  bufferMargin: number;
  stylometryProbability: number | null;
  qualityScoreNormalized: number | null;
}

export interface ModeDetectionResult {
  aggregation: AggregationResponse;
  segments: SegmentResponse[];
  segmentCount: number;
}

export interface ComparisonResult {
  probabilityDiff: number;
  consistencyScore: number;
  divergentRegions: Array<{
    paragraphSegmentId: number;
    sentenceSegmentId: number;
    probabilityDiff: number;
    paragraphProb: number;
    sentenceProb: number;
    textPreview: string;
  }>;
}

export interface DualDetectionResult {
  paragraph: ModeDetectionResult;
  sentence: ModeDetectionResult;
  comparison: ComparisonResult;
  /** Fused aggregation combining paragraph and sentence results (weight: paragraph 0.6 + sentence 0.4) */
  fusedAggregation?: AggregationResponse;
}

// Content Filter Types
export type ParagraphCategory = 'body' | 'title' | 'toc' | 'reference' | 'auxiliary' | 'noise';

export interface ParagraphClassification {
  index: number;
  category: ParagraphCategory;
  confidence: number;
  reason: string;
}

export interface FilterSummary {
  totalParagraphs: number;
  bodyCount: number;
  filteredCount: number;
  filteredByRule: number;
  filteredByLlm: number;
  classifications: ParagraphClassification[];
}

export interface DetectResponse {
  aggregation: AggregationResponse;
  segments: SegmentResponse[];
  preprocessSummary: {
    language: string;
    chunks: number;
    redacted: number;
  };
  cost: {
    tokens: number;
    latencyMs: number;
  };
  version: string;
  requestId: string;
  dualDetection: DualDetectionResult | null;
  filterSummary?: FilterSummary;
}

// Request Types
export interface DetectTextRequest {
  text: string;
  usePerplexity: boolean;
  useStylometry: boolean;
  sensitivity: string;
  provider: string | null;
  dualMode: boolean;
}

// UI State Types
export type DecisionType = 'pass' | 'review' | 'flag';

export interface UIState {
  isLoading: boolean;
  loadingText: string;
  sensitivityOpen: boolean;
  providerOpen: boolean;
  settingsOpen: boolean;
}
