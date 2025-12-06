from typing import List, Optional, Dict, Any
from pydantic import BaseModel, Field


class PreprocessOptions(BaseModel):
    stripHtml: bool = False
    redactPII: bool = False
    normalizePunctuation: bool = True
    autoLanguage: bool = True
    chunkSizeTokens: int = 1500
    overlapTokens: int = 150
    alignToParagraphs: bool = True
    paragraphMergeMinChars: int = 200
    paragraphSplitMaxSentenceLen: int = 120


class ChunkingOptions(BaseModel):
    chunkSizeTokens: int = 1500
    overlapTokens: int = 150


class DetectRequest(BaseModel):
    text: str
    language: Optional[str] = None
    genre: Optional[str] = None
    providers: List[str] = Field(default_factory=list)
    usePerplexity: bool = True
    useStylometry: bool = True
    preprocessOptions: PreprocessOptions = Field(default_factory=PreprocessOptions)
    chunking: ChunkingOptions = Field(default_factory=ChunkingOptions)
    sensitivity: str = Field(default="medium")


class SignalStylometry(BaseModel):
    ttr: float
    avgSentenceLen: float
    functionWordRatio: Optional[float] = None
    repeatRatio: Optional[float] = None
    punctuationRatio: Optional[float] = None


class SignalPerplexity(BaseModel):
    ppl: Optional[float] = None
    z: Optional[float] = None


class SignalLLMJudgment(BaseModel):
    prob: Optional[float] = None
    models: List[str] = Field(default_factory=list)


class SegmentSignals(BaseModel):
    llmJudgment: SignalLLMJudgment
    perplexity: SignalPerplexity
    stylometry: SignalStylometry


class SegmentOffsets(BaseModel):
    start: int
    end: int


class SegmentResponse(BaseModel):
    chunkId: int
    language: str
    offsets: SegmentOffsets
    aiProbability: float
    confidence: float
    signals: SegmentSignals
    explanations: List[str]


class AggregationThresholds(BaseModel):
    low: float = 0.65
    medium: float = 0.75
    high: float = 0.85
    veryHigh: float = 0.90


class AggregationResponse(BaseModel):
    overallProbability: float
    overallConfidence: float
    method: str
    thresholds: AggregationThresholds
    rubricVersion: str
    decision: str
    bufferMargin: float
    stylometryProbability: Optional[float] = None
    qualityScoreNormalized: Optional[float] = None
    blockWeights: Optional[Dict[str, float]] = None
    dimensionScores: Optional[Dict[str, int]] = None

class ReviewLogItem(BaseModel):
    ts: str
    requestId: str
    overallProbability: float
    overallConfidence: float
    decision: str
    label: Optional[int] = None
    notes: Optional[str] = None

class ReviewSubmitRequest(BaseModel):
    requestId: str
    decision: str
    overallProbability: float
    overallConfidence: float
    label: Optional[int] = None
    notes: Optional[str] = None

class ReviewSubmitResponse(BaseModel):
    ok: bool
    total: int
    passCount: int
    reviewCount: int
    flagCount: int

class ReviewSummaryResponse(BaseModel):
    total: int
    labeled: int
    tp: int
    tn: int
    fp: int
    fn: int
    accuracy: float
    precision: float
    recall: float
    f1: float

class PromptVariant(BaseModel):
    id: str
    name: str
    style: str
    schemaVersion: str

class PromptVariantsResponse(BaseModel):
    items: List[PromptVariant]

class ConsistencyCheckRequest(BaseModel):
    segments: List[SegmentResponse]

class ConsistencyIssue(BaseModel):
    segmentId: int
    type: str
    message: str

class ConsistencyCheckResponse(BaseModel):
    ok: bool
    issues: List[ConsistencyIssue]

class PaperAnalyzeRequest(BaseModel):
    text: str
    language: Optional[str] = None
    genre: Optional[str] = None
    rounds: int = 6
    useLLM: bool = True

class ReadabilityScores(BaseModel):
    fluency: float
    clarity: float
    cohesion: float

class SuggestionItem(BaseModel):
    title: str
    detail: str

class MultiRoundDetail(BaseModel):
    round: int
    probability: float
    confidence: float
    templateId: Optional[str] = None
    ts: Optional[str] = None

class MultiRoundSummary(BaseModel):
    rounds: int
    avgProbability: float
    avgConfidence: float
    variance: float
    details: List[MultiRoundDetail]
    trimmedAvgProbability: Optional[float] = None
    trimmedAvgConfidence: Optional[float] = None

class PaperAnalyzeResponse(BaseModel):
    aggregation: AggregationResponse
    readability: ReadabilityScores
    multiRound: MultiRoundSummary
    suggestions: List[SuggestionItem]

class HistoryItem(BaseModel):
    id: str
    ts: str
    reqParams: Dict[str, Any]
    aggregation: AggregationResponse
    multiRound: Optional[MultiRoundSummary] = None

class HistorySaveRequest(BaseModel):
    id: str
    reqParams: Dict[str, Any]
    aggregation: AggregationResponse
    multiRound: Optional[MultiRoundSummary] = None

class HistorySaveResponse(BaseModel):
    ok: bool
    total: int

class HistoryListResponse(BaseModel):
    items: List[HistoryItem]


class PreprocessSummary(BaseModel):
    language: str
    chunks: int
    redacted: int = 0


class CostBreakdown(BaseModel):
    tokens: int
    latencyMs: int
    providerBreakdown: Dict[str, Any] = Field(default_factory=dict)


class DetectResponse(BaseModel):
    aggregation: AggregationResponse
    segments: List[SegmentResponse]
    preprocessSummary: PreprocessSummary
    cost: CostBreakdown
    version: str
    requestId: str


class PreprocessUploadResponse(BaseModel):
    normalizedText: str
    preprocessSummary: PreprocessSummary
    segments: List[SegmentResponse]
    structuredNodes: List[Dict[str, Any]] = Field(default_factory=list)
    formattedText: Optional[str] = None
    formatSummary: Optional[Dict[str, Any]] = None
    mapping: Optional[Dict[str, Any]] = None
    comparison: Optional[Dict[str, Any]] = None


class BatchItemRequest(BaseModel):
    id: str
    text: str
    language: Optional[str] = None
    genre: Optional[str] = None
    providers: List[str] = Field(default_factory=list)
    usePerplexity: bool = True
    useStylometry: bool = True
    preprocessOptions: PreprocessOptions = Field(default_factory=PreprocessOptions)
    chunking: ChunkingOptions = Field(default_factory=ChunkingOptions)
    sensitivity: str = Field(default="medium")


class BatchItemResponse(BaseModel):
    id: str
    aggregation: AggregationResponse
    segments: List[SegmentResponse]
    preprocessSummary: PreprocessSummary
    cost: CostBreakdown
    version: str


class BatchSummary(BaseModel):
    count: int
    failCount: int
    avgProbability: float
    p95Probability: float


class BatchDetectRequest(BaseModel):
    items: List[BatchItemRequest]
    parallel: Optional[int] = 4


class BatchDetectResponse(BaseModel):
    items: List[BatchItemResponse]
    summary: BatchSummary


class CalibrateItem(BaseModel):
    prob: float
    label: int


class CalibrateRequest(BaseModel):
    items: List[CalibrateItem]


class CalibrateResponse(BaseModel):
    ok: bool
    version: str
    A: float = 0.0
    B: float = 0.0
