// Comparison Logic
// Compares paragraph and sentence detection results

use crate::models::{ComparisonResult, DivergentRegion, SegmentResponse};

fn safe_preview(text: &str, start: i32, end: i32, max_bytes: usize) -> String {
    if start < 0 || end <= start || max_bytes == 0 {
        return String::new();
    }

    let len = text.len();
    let mut s = (start as usize).min(len);
    let e_limit = (end as usize).min(len);
    if s >= e_limit {
        return String::new();
    }

    // Ensure start is on a char boundary (should already be true for our offsets).
    while s < e_limit && !text.is_char_boundary(s) {
        s += 1;
    }
    if s >= e_limit {
        return String::new();
    }

    // Take a byte-bounded preview and then snap end backward to a char boundary.
    let mut e = (s + max_bytes).min(e_limit);
    while e > s && !text.is_char_boundary(e) {
        e -= 1;
    }

    text.get(s..e).unwrap_or("").to_string()
}

/// Compare paragraph and sentence detection results
pub fn compare_dual_mode_results(
    para_segments: &[SegmentResponse],
    sent_segments: &[SegmentResponse],
    text: &str,
    diff_threshold: f64,
) -> ComparisonResult {
    if para_segments.is_empty() || sent_segments.is_empty() {
        return ComparisonResult {
            probability_diff: 0.0,
            consistency_score: 1.0,
            divergent_regions: vec![],
        };
    }

    // Calculate overall probability difference
    let para_avg: f64 =
        para_segments.iter().map(|s| s.ai_probability).sum::<f64>() / para_segments.len() as f64;
    let sent_avg: f64 =
        sent_segments.iter().map(|s| s.ai_probability).sum::<f64>() / sent_segments.len() as f64;
    let probability_diff = (para_avg - sent_avg).abs();

    // Find divergent regions
    let mut divergent_regions = Vec::new();
    let mut consistent_count = 0;
    let mut total_comparisons = 0;

    for p_seg in para_segments {
        let p_start = p_seg.offsets.start;
        let p_end = p_seg.offsets.end;
        let p_prob = p_seg.ai_probability;

        for s_seg in sent_segments {
            let s_start = s_seg.offsets.start;
            let s_end = s_seg.offsets.end;
            let s_prob = s_seg.ai_probability;

            // Check for overlap
            let overlap_start = p_start.max(s_start);
            let overlap_end = p_end.min(s_end);
            let overlap_len = (overlap_end - overlap_start).max(0);

            if overlap_len > 0 {
                let p_coverage = overlap_len as f64 / (p_end - p_start).max(1) as f64;
                let s_coverage = overlap_len as f64 / (s_end - s_start).max(1) as f64;

                if p_coverage > 0.5 && s_coverage > 0.5 {
                    total_comparisons += 1;
                    let p_direction = if p_prob > 0.5 { 1 } else { 0 };
                    let s_direction = if s_prob > 0.5 { 1 } else { 0 };

                    if p_direction == s_direction {
                        consistent_count += 1;
                    }

                    let prob_diff = (p_prob - s_prob).abs();
                    if prob_diff > diff_threshold {
                        let preview_end = (overlap_start + 100).min(overlap_end);
                        let mut text_preview =
                            safe_preview(text, overlap_start, preview_end, 100);
                        if preview_end < overlap_end {
                            text_preview.push_str("...");
                        }

                        divergent_regions.push(DivergentRegion {
                            paragraph_segment_id: p_seg.chunk_id,
                            sentence_segment_id: s_seg.chunk_id,
                            probability_diff: (prob_diff * 10000.0).round() / 10000.0,
                            paragraph_prob: (p_prob * 10000.0).round() / 10000.0,
                            sentence_prob: (s_prob * 10000.0).round() / 10000.0,
                            text_preview,
                        });
                    }
                }
            }
        }
    }

    let consistency_score = if total_comparisons > 0 {
        consistent_count as f64 / total_comparisons as f64
    } else {
        1.0
    };

    ComparisonResult {
        probability_diff: (probability_diff * 10000.0).round() / 10000.0,
        consistency_score: (consistency_score * 10000.0).round() / 10000.0,
        divergent_regions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SegmentOffsets, SegmentSignals};

    #[test]
    fn test_compare_dual_mode_results_cjk_preview_does_not_panic() {
        // "中" is 3 bytes in UTF-8; byte-based preview lengths can cut mid-codepoint.
        let text = "中".repeat(200);
        let end = text.len() as i32;

        let para = SegmentResponse {
            chunk_id: 0,
            language: "zh".to_string(),
            offsets: SegmentOffsets { start: 0, end },
            ai_probability: 0.10,
            confidence: 0.8,
            signals: SegmentSignals::default(),
            explanations: vec![],
        };

        let sent = SegmentResponse {
            chunk_id: 0,
            language: "zh".to_string(),
            offsets: SegmentOffsets { start: 0, end },
            ai_probability: 0.90,
            confidence: 0.8,
            signals: SegmentSignals::default(),
            explanations: vec![],
        };

        let result = compare_dual_mode_results(&[para], &[sent], &text, 0.20);
        assert_eq!(result.divergent_regions.len(), 1);
        assert!(!result.divergent_regions[0].text_preview.is_empty());
        assert!(result.divergent_regions[0].text_preview.ends_with("..."));
    }
}
