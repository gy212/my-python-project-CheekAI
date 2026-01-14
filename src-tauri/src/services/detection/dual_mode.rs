// Dual Mode Detection
// Combines paragraph and sentence detection modes
// - Paragraph mode: LLM analysis (default OpenAI GPT-5.2)
// - Sentence mode: LLM analysis (default OpenAI GPT-5.2)
// - Result fusion: Weighted combination (paragraph 0.6 + sentence 0.4)

use crate::models::{AggregationResponse, DualDetectionResult, ModeDetectionResult};
use crate::services::text_processor::{build_paragraph_blocks, build_sentence_blocks};
use tracing::info;

use super::aggregation::aggregate_segments;
use super::comparison::compare_dual_mode_results;
use super::llm_analyzer::build_segments_with_llm;
use super::segment_builder::build_segments;

/// Weight for paragraph mode in fusion
const PARAGRAPH_WEIGHT: f64 = 0.6;
/// Weight for sentence mode in fusion
const SENTENCE_WEIGHT: f64 = 0.4;

/// Perform dual-mode detection (paragraph + sentence) - sync version
pub fn dual_mode_detect(
    text: &str,
    language: &str,
    use_perplexity: bool,
    use_stylometry: bool,
) -> DualDetectionResult {
    // Paragraph mode
    let para_blocks = build_paragraph_blocks(text);
    let para_segments = build_segments(text, language, &para_blocks, use_perplexity, use_stylometry);
    let para_aggregation = aggregate_segments(&para_segments);

    // Sentence mode
    let sent_blocks = build_sentence_blocks(text, 50, 200, 300);
    let sent_segments = build_segments(text, language, &sent_blocks, use_perplexity, use_stylometry);
    let sent_aggregation = aggregate_segments(&sent_segments);

    // Compare results
    let comparison = compare_dual_mode_results(&para_segments, &sent_segments, text, 0.20);

    // Create fused aggregation for sync version too
    let fused_aggregation = fuse_aggregations(&para_aggregation, &sent_aggregation);

    DualDetectionResult {
        paragraph: ModeDetectionResult {
            aggregation: para_aggregation,
            segments: para_segments.clone(),
            segment_count: para_segments.len() as i32,
        },
        sentence: ModeDetectionResult {
            aggregation: sent_aggregation,
            segments: sent_segments.clone(),
            segment_count: sent_segments.len() as i32,
        },
        comparison,
        fused_aggregation: Some(fused_aggregation),
    }
}

/// Perform dual-mode detection with LLM (async version)
/// Uses the selected provider, defaulting to OpenAI (GPT-5.2)
/// Processes both modes in parallel for efficiency.
pub async fn dual_mode_detect_with_llm(
    text: &str,
    language: &str,
    use_perplexity: bool,
    use_stylometry: bool,
    provider: Option<&str>,
) -> DualDetectionResult {
    info!("[DUAL_MODE] Starting LLM-powered dual mode detection");
    info!(
        "[DUAL_MODE] Text length: {} chars ({} bytes), language: {}",
        text.chars().count(),
        text.len(),
        language
    );
    
    // Build blocks first
    let para_blocks = build_paragraph_blocks(text);
    let sent_blocks = build_sentence_blocks(text, 50, 200, 300);
    
    info!(
        "[DUAL_MODE] Paragraph blocks: {}, Sentence blocks: {}",
        para_blocks.len(),
        sent_blocks.len()
    );

    // Run paragraph and sentence detection in parallel
    let para_future =
        build_segments_with_llm(text, language, &para_blocks, use_perplexity, use_stylometry, provider);
    let sent_future =
        build_segments_with_llm(text, language, &sent_blocks, use_perplexity, use_stylometry, provider);

    // Execute both in parallel
    info!("[DUAL_MODE] Starting parallel LLM calls...");
    let (para_segments, sent_segments) = tokio::join!(para_future, sent_future);
    info!(
        "[DUAL_MODE] Parallel calls completed. Paragraph segments: {}, Sentence segments: {}",
        para_segments.len(),
        sent_segments.len()
    );

    // Aggregate each mode
    let para_aggregation = aggregate_segments(&para_segments);
    let sent_aggregation = aggregate_segments(&sent_segments);
    
    info!(
        "[DUAL_MODE] Paragraph prob: {:.2}, Sentence prob: {:.2}",
        para_aggregation.overall_probability,
        sent_aggregation.overall_probability
    );

    // Compare results
    let comparison = compare_dual_mode_results(&para_segments, &sent_segments, text, 0.20);

    // Create fused aggregation
    let fused_aggregation = fuse_aggregations(&para_aggregation, &sent_aggregation);

    DualDetectionResult {
        paragraph: ModeDetectionResult {
            aggregation: para_aggregation,
            segments: para_segments.clone(),
            segment_count: para_segments.len() as i32,
        },
        sentence: ModeDetectionResult {
            aggregation: sent_aggregation,
            segments: sent_segments.clone(),
            segment_count: sent_segments.len() as i32,
        },
        comparison,
        fused_aggregation: Some(fused_aggregation),
    }
}

/// Fuse paragraph and sentence aggregations with weighted combination
/// Paragraph weight: 0.6, Sentence weight: 0.4
fn fuse_aggregations(
    para_agg: &AggregationResponse,
    sent_agg: &AggregationResponse,
) -> AggregationResponse {
    // Weighted fusion of probabilities
    let fused_probability = if sent_agg.overall_probability > 0.0 {
        para_agg.overall_probability * PARAGRAPH_WEIGHT 
            + sent_agg.overall_probability * SENTENCE_WEIGHT
    } else {
        // If no sentence results, use paragraph only
        para_agg.overall_probability
    };

    // Weighted fusion of confidence
    let fused_confidence = if sent_agg.overall_confidence > 0.0 {
        para_agg.overall_confidence * PARAGRAPH_WEIGHT 
            + sent_agg.overall_confidence * SENTENCE_WEIGHT
    } else {
        para_agg.overall_confidence
    };

    // Derive decision based on fused probability
    let decision = super::aggregation::derive_decision(
        fused_probability,
        &para_agg.thresholds,
        para_agg.buffer_margin,
    );

    AggregationResponse {
        overall_probability: fused_probability.clamp(0.0, 1.0),
        overall_confidence: fused_confidence.clamp(0.0, 1.0),
        method: "dual_mode_fusion".to_string(),
        thresholds: para_agg.thresholds.clone(),
        rubric_version: para_agg.rubric_version.clone(),
        decision,
        buffer_margin: para_agg.buffer_margin,
        stylometry_probability: para_agg.stylometry_probability,
        quality_score_normalized: para_agg.quality_score_normalized,
        block_weights: None,
        dimension_scores: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_mode_detect() {
        let text = "这是第一段测试文本。\n\n这是第二段测试文本。";
        let result = dual_mode_detect(text, "zh", true, true);
        assert!(result.paragraph.segment_count > 0);
        assert!(result.sentence.segment_count > 0);
    }
}
