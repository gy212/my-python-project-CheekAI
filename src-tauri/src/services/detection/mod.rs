// Detection Module
// AI text detection core logic organized into specialized submodules:
// - segment_builder: Builds detection segments from text blocks
// - aggregation: Aggregates segment results into overall detection result
// - comparison: Compares paragraph and sentence detection results
// - dual_mode: Combines paragraph and sentence detection modes
// - llm_analyzer: Handles LLM-based text analysis (batch GLM + filtered DeepSeek)
// - content_filter: Hybrid filtering to identify non-body content

pub mod segment_builder;
pub mod aggregation;
pub mod comparison;
pub mod dual_mode;
pub mod llm_analyzer;
pub mod content_filter;
pub mod sensitivity;
pub mod subject_catalog;

// Re-export commonly used functions
pub use segment_builder::{
    build_segments,
    build_segments_with_profile,
    make_segment,
    make_segment_with_profile,
    estimate_perplexity,
};
pub use aggregation::{aggregate_segments, derive_decision, contrast_sharpen_segments};
pub use comparison::compare_dual_mode_results;
pub use dual_mode::{dual_mode_detect, dual_mode_detect_with_llm};
pub use llm_analyzer::{
    build_document_profile,
    build_segments_with_llm,
    build_segments_with_llm_progress,
    build_paragraphs_batch_with_glm,
    build_sentences_filtered_with_deepseek,
};
pub use subject_catalog::{catalog_year, profile_validity, validate_document_profile, ProfileValidity};
pub use sensitivity::{
    apply_segment_decisions,
    decide_overall,
    decide_segment,
    decision_thresholds,
    DetectionSensitivity,
    EvidenceSummary,
    summarize_evidence,
};
pub use content_filter::{
    filter_paragraphs,
    FilterSummary,
    ParagraphCategory,
    ParagraphClassification,
};
