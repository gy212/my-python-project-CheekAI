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
use super::llm_analyzer::{build_document_profile, build_segments_with_llm_progress};
use super::segment_builder::build_segments;
use super::sensitivity::apply_segment_decisions;

/// Weight for paragraph mode in fusion
const PARAGRAPH_WEIGHT: f64 = 0.6;
/// Weight for sentence mode in fusion
const SENTENCE_WEIGHT: f64 = 0.4;
const DECISION_MARGIN: f64 = 0.03;

/// Perform dual-mode detection (paragraph + sentence) - sync version
pub fn dual_mode_detect(
    text: &str,
    language: &str,
    use_perplexity: bool,
    use_stylometry: bool,
    sensitivity: &str,
) -> DualDetectionResult {
    // Paragraph mode
    let para_blocks = build_paragraph_blocks(text);
    let mut para_segments = build_segments(text, language, &para_blocks, use_perplexity, use_stylometry);
    apply_segment_decisions(&mut para_segments, sensitivity, DECISION_MARGIN);
    let para_aggregation = aggregate_segments(&para_segments, sensitivity);

    // Sentence mode
    let sent_blocks = build_sentence_blocks(text, 50, 200, 300);
    let mut sent_segments = build_segments(text, language, &sent_blocks, use_perplexity, use_stylometry);
    apply_segment_decisions(&mut sent_segments, sensitivity, DECISION_MARGIN);
    let sent_aggregation = aggregate_segments(&sent_segments, sensitivity);

    // Compare results
    let comparison = compare_dual_mode_results(&para_segments, &sent_segments, text, 0.20);

    // Create fused aggregation for sync version too
    let fused_aggregation = fuse_aggregations(&para_aggregation, &sent_aggregation, &para_segments, sensitivity);

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
        filter_summary: None,
        document_profile: None,
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
    sensitivity: &str,
) -> DualDetectionResult {
    info!("[DUAL_MODE] Starting LLM-powered dual mode detection");
    info!(
        "[DUAL_MODE] Text length: {} chars ({} bytes), language: {}",
        text.chars().count(),
        text.len(),
        language
    );
    
    // Build blocks first
    let para_blocks_raw = build_paragraph_blocks(text);
    let (para_blocks, filter_summary) = crate::services::filter_paragraphs(&para_blocks_raw, provider).await;
    let sent_blocks = crate::services::build_sentence_blocks_smart_in_paragraphs(
        text,
        language,
        &para_blocks,
        200,
        300,
        provider,
    )
    .await;
    
    info!(
        "[DUAL_MODE] Paragraph blocks: {}, Sentence blocks: {}",
        para_blocks.len(),
        sent_blocks.len()
    );

    let document_profile = build_document_profile(text, &para_blocks_raw, provider).await;

    // Run paragraph and sentence detection in parallel
    let para_future = build_segments_with_llm_progress(
        text,
        language,
        &para_blocks,
        use_perplexity,
        use_stylometry,
        provider,
        Some(&para_blocks_raw),
        document_profile.as_ref(),
        |_, _| {},
    );
    let sent_future = build_segments_with_llm_progress(
        text,
        language,
        &sent_blocks,
        use_perplexity,
        use_stylometry,
        provider,
        None,
        document_profile.as_ref(),
        |_, _| {},
    );

    // Execute both in parallel
    info!("[DUAL_MODE] Starting parallel LLM calls...");
    let (mut para_segments, mut sent_segments) = tokio::join!(para_future, sent_future);
    info!(
        "[DUAL_MODE] Parallel calls completed. Paragraph segments: {}, Sentence segments: {}",
        para_segments.len(),
        sent_segments.len()
    );

    // Aggregate each mode
    apply_segment_decisions(&mut para_segments, sensitivity, DECISION_MARGIN);
    apply_segment_decisions(&mut sent_segments, sensitivity, DECISION_MARGIN);
    let para_aggregation = aggregate_segments(&para_segments, sensitivity);
    let sent_aggregation = aggregate_segments(&sent_segments, sensitivity);
    
    info!(
        "[DUAL_MODE] Paragraph prob: {:.2}, Sentence prob: {:.2}",
        para_aggregation.overall_probability,
        sent_aggregation.overall_probability
    );

    // Compare results
    let comparison = compare_dual_mode_results(&para_segments, &sent_segments, text, 0.20);

    // Create fused aggregation
    let fused_aggregation = fuse_aggregations(&para_aggregation, &sent_aggregation, &para_segments, sensitivity);

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
        filter_summary: Some(filter_summary),
        document_profile,
    }
}

/// Fuse paragraph and sentence aggregations with weighted combination
/// Paragraph weight: 0.6, Sentence weight: 0.4
fn fuse_aggregations(
    para_agg: &AggregationResponse,
    sent_agg: &AggregationResponse,
    segments_for_gate: &[crate::models::SegmentResponse],
    sensitivity: &str,
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
    let overall_uncertainty = segments_for_gate
        .iter()
        .map(|s| s.uncertainty)
        .sum::<f64>()
        / segments_for_gate.len().max(1) as f64;
    let decision = super::aggregation::derive_decision(
        fused_probability,
        overall_uncertainty,
        segments_for_gate,
        sensitivity,
        para_agg.buffer_margin,
    );

    AggregationResponse {
        overall_probability: fused_probability.clamp(0.0, 1.0),
        overall_confidence: fused_confidence.clamp(0.0, 1.0),
        method: "dual_mode_fusion".to_string(),
        thresholds: para_agg.thresholds.clone(),
        decision_thresholds: para_agg.decision_thresholds.clone(),
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
        let result = dual_mode_detect(text, "zh", true, true, "medium");
        assert!(result.paragraph.segment_count > 0);
        assert!(result.sentence.segment_count > 0);
    }
}
