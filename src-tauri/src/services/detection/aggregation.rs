// Aggregation Logic
// Aggregates segment results into overall detection result

use crate::models::{AggregationResponse, AggregationThresholds, SegmentResponse};
use super::sensitivity::{decide_overall, decision_thresholds};

const RUBRIC_VERSION: &str = "rubric-v1.2";
const DEFAULT_BUFFER_MARGIN: f64 = 0.03;

/// Aggregate segments into overall result
/// Uses confidence-weighted aggregation with robust statistics
pub fn aggregate_segments(segments: &[SegmentResponse], sensitivity: &str) -> AggregationResponse {
    if segments.is_empty() {
        let decision_thresholds = decision_thresholds(sensitivity);
        return AggregationResponse {
            overall_probability: 0.0,
            overall_confidence: 0.0,
            method: "weighted".to_string(),
            thresholds: AggregationThresholds::default(),
            decision_thresholds,
            rubric_version: RUBRIC_VERSION.to_string(),
            decision: "pass".to_string(),
            buffer_margin: DEFAULT_BUFFER_MARGIN,
            stylometry_probability: None,
            quality_score_normalized: None,
            block_weights: None,
            dimension_scores: None,
        };
    }

    // Calculate weights: sqrt(length) * confidence
    // This prevents long segments from dominating while incorporating confidence
    let weights: Vec<f64> = segments
        .iter()
        .map(|s| {
            let len = (s.offsets.end - s.offsets.start).max(50) as f64;
            len.sqrt() * s.confidence.max(0.3)  // min confidence floor of 0.3
        })
        .collect();
    let total: f64 = weights.iter().sum();

    // Weighted average probability
    let weighted_prob: f64 = segments
        .iter()
        .zip(weights.iter())
        .map(|(s, w)| s.raw_probability * w)
        .sum::<f64>()
        / total.max(1.0);

    // Also compute trimmed mean (remove top/bottom 10%) for robustness
    let trimmed_prob = if segments.len() >= 5 {
        let mut probs: Vec<f64> = segments.iter().map(|s| s.raw_probability).collect();
        probs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let trim_count = (probs.len() as f64 * 0.1).ceil() as usize;
        let trimmed: Vec<f64> = probs[trim_count..probs.len() - trim_count].to_vec();
        if trimmed.is_empty() {
            weighted_prob
        } else {
            trimmed.iter().sum::<f64>() / trimmed.len() as f64
        }
    } else {
        weighted_prob
    };

    // Blend weighted and trimmed (70% weighted, 30% trimmed for stability)
    let overall = 0.7 * weighted_prob + 0.3 * trimmed_prob;

    // Weighted confidence
    let confidence: f64 = segments
        .iter()
        .zip(weights.iter())
        .map(|(s, w)| s.confidence * w)
        .sum::<f64>()
        / total.max(1.0);

    let overall_uncertainty: f64 = segments
        .iter()
        .zip(weights.iter())
        .map(|(s, w)| s.uncertainty * w)
        .sum::<f64>()
        / total.max(1.0);

    let thresholds = AggregationThresholds::default();
    let decision_thresholds = decision_thresholds(sensitivity);
    let decision = decide_overall(
        overall.clamp(0.0, 1.0),
        overall_uncertainty.clamp(0.0, 1.0),
        segments,
        sensitivity,
        DEFAULT_BUFFER_MARGIN,
    );

    AggregationResponse {
        overall_probability: overall.clamp(0.0, 1.0),
        overall_confidence: confidence.clamp(0.0, 1.0),
        method: "confidence_weighted".to_string(),
        thresholds,
        decision_thresholds,
        rubric_version: RUBRIC_VERSION.to_string(),
        decision,
        buffer_margin: DEFAULT_BUFFER_MARGIN,
        stylometry_probability: Some(weighted_prob),
        quality_score_normalized: Some(0.5 + (confidence - 0.5) * 0.6),
        block_weights: None,
        dimension_scores: None,
    }
}

/// Derive decision from aggregated probability and uncertainty (delegates to sensitivity gates)
pub fn derive_decision(
    prob: f64,
    overall_uncertainty: f64,
    segments: &[SegmentResponse],
    sensitivity: &str,
    margin: f64,
) -> String {
    decide_overall(prob, overall_uncertainty, segments, sensitivity, margin)
}

/// Apply contrast sharpening to segments
pub fn contrast_sharpen_segments(segments: &mut [SegmentResponse]) {
    if segments.len() < 4 {
        return;
    }

    let probs: Vec<f64> = segments.iter().map(|s| s.raw_probability).collect();
    let mut sorted = probs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = sorted[sorted.len() / 2];
    let q1 = sorted[sorted.len() / 4];
    let q3 = sorted[(sorted.len() * 3) / 4];
    let iqr = (q3 - q1).max(1e-6);

    let z: Vec<f64> = probs.iter().map(|p| (p - median) / (iqr / 1.349)).collect();

    let base_gamma = 1.45;

    let doc_std = std_dev(&probs);
    let flat_boost = 1.0 + ((0.06 - doc_std) * 10.0).max(0.0);
    let gamma = (base_gamma * flat_boost).min(2.5);

    let logits: Vec<f64> = probs.iter().map(|&p| logit_safe(p)).collect();
    let confs: Vec<f64> = segments.iter().map(|s| s.confidence).collect();

    let logits_prime: Vec<f64> = logits
        .iter()
        .zip(z.iter())
        .zip(confs.iter())
        .map(|((l, z_i), c)| l + gamma * z_i * (0.6 + 0.4 * c.clamp(0.3, 0.92)))
        .collect();

    let target_mean: f64 = probs.iter().sum::<f64>() / probs.len() as f64;

    // Binary search for shift constant
    let mut lo = -6.0;
    let mut hi = 6.0;
    for _ in 0..28 {
        let mid = (lo + hi) / 2.0;
        let m = mean_after_shift(&logits_prime, mid);
        if m > target_mean {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let c = (lo + hi) / 2.0;

    // Apply sharpening
    for (seg, lp) in segments.iter_mut().zip(logits_prime.iter()) {
        let x = lp - c;
        let new_p = sigmoid_clamped(x);
        let final_p = if seg.confidence < 0.5 {
            0.8 * seg.raw_probability + 0.2 * new_p
        } else {
            new_p
        };
        seg.raw_probability = final_p.clamp(0.02, 0.98);
    }
}

// Helper functions for contrast sharpening

fn logit_safe(p: f64) -> f64 {
    let p = p.clamp(1e-6, 1.0 - 1e-6);
    (p / (1.0 - p)).ln()
}

fn sigmoid_clamped(x: f64) -> f64 {
    if x > 40.0 {
        1.0
    } else if x < -40.0 {
        0.0
    } else {
        1.0 / (1.0 + (-x).exp())
    }
}

fn mean_after_shift(logits: &[f64], c: f64) -> f64 {
    logits.iter().map(|l| sigmoid_clamped(l - c)).sum::<f64>() / logits.len().max(1) as f64
}

fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_decision() {
        let segments: Vec<SegmentResponse> = Vec::new();
        let decision = derive_decision(0.7, 0.2, &segments, "medium", 0.03);
        assert!(decision == "review" || decision == "flag");
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate_segments(&[], "medium");
        assert_eq!(result.overall_probability, 0.0);
        assert_eq!(result.decision, "pass");
    }
}
