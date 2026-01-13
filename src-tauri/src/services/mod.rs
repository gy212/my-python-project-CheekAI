// CheekAI Core Services
// Migrated from Python backend

pub mod text_processor;
pub mod config_store;
pub mod providers;
pub mod detection;
pub mod sentence_segmenter;

pub use text_processor::*;
pub use config_store::*;
pub use providers::*;
pub use sentence_segmenter::*;

// Re-export detection module functions
pub use detection::{
    build_segments,
    make_segment,
    estimate_perplexity,
    aggregate_segments,
    derive_decision,
    contrast_sharpen_segments,
    compare_dual_mode_results,
    dual_mode_detect,
    dual_mode_detect_with_llm,
    build_segments_with_llm,
};
