// Segment Builder
// Builds detection segments from text blocks
//
// Algorithm v2: Uses soft thresholds (sigmoid) and logit-space scoring
// for continuous, non-discrete probability outputs.

use crate::models::{
    DocumentProfile, SegmentOffsets, SegmentResponse, SegmentSignals, SignalLLMJudgment,
    SignalPerplexity, SignalStylometry,
};
use crate::services::text_processor::compute_stylometry;
use super::subject_catalog::{is_academic_profile, profile_validity, ProfileValidity};
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

// ============================================================================
// Soft threshold functions for continuous scoring
// ============================================================================

/// Sigmoid function: smooth transition around center
/// k controls steepness (smaller = steeper)
#[inline]
fn sigmoid(x: f64, center: f64, k: f64) -> f64 {
    1.0 / (1.0 + ((x - center) / k).exp())
}

/// Inverse sigmoid: 1 - sigmoid (for "greater than" thresholds)
#[inline]
fn sigmoid_inv(x: f64, center: f64, k: f64) -> f64 {
    1.0 - sigmoid(x, center, k)
}

/// Convert probability to logit (log-odds)
#[inline]
#[allow(dead_code)]
fn to_logit(p: f64) -> f64 {
    let p_clamped = p.clamp(0.001, 0.999);
    (p_clamped / (1.0 - p_clamped)).ln()
}

/// Convert logit back to probability
#[inline]
fn from_logit(logit: f64) -> f64 {
    1.0 / (1.0 + (-logit).exp())
}

/// Deterministic hash-based noise for reproducibility
fn deterministic_noise(text: &str, seed: u64) -> f64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    seed.hash(&mut hasher);
    let hash = hasher.finish();
    // Map to [-0.5, 0.5] range
    ((hash % 10000) as f64 / 10000.0) - 0.5
}

fn profile_strength(profile: Option<&DocumentProfile>) -> f64 {
    let Some(profile) = profile else {
        return 0.0;
    };
    if !is_academic_profile(profile) {
        return 0.0;
    }
    match profile_validity(profile) {
        ProfileValidity::Valid => 1.0,
        ProfileValidity::Partial => 0.6,
        ProfileValidity::Invalid => 0.0,
    }
}

fn is_cjk_language(language: &str) -> bool {
    let lang = language.trim().to_ascii_lowercase();
    lang.starts_with("zh") || lang.starts_with("ja") || lang.starts_with("ko")
}

fn citation_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(\[[0-9]{1,3}\])|(\([A-Za-z][A-Za-z\s\.-]+,\s*\d{4}[a-z]?\))")
            .expect("citation regex")
    })
}

fn section_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?m)^(摘要|引言|绪论|方法|结果|讨论|结论|致谢|参考文献|Abstract|Introduction|Methods?|Results?|Discussion|Conclusion|Acknowledg(e)?ments?|References?)",
        )
        .expect("section regex")
    })
}

fn figure_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(图\s?\d+|表\s?\d+|figure\s?\d+|table\s?\d+|equation\s?\(?\d+\)?)")
            .expect("figure regex")
    })
}

fn academic_anchor_strength(text: &str) -> f64 {
    let mut strength: f64 = 0.0;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    if citation_re().is_match(trimmed) || trimmed.contains("et al.") || trimmed.contains("DOI") || trimmed.contains("doi") {
        strength += 0.4;
    }
    if section_re().is_match(trimmed) {
        strength += 0.3;
    }
    if figure_re().is_match(trimmed) {
        strength += 0.3;
    }
    strength.min(1.0)
}

/// Build segments from text blocks
pub fn build_segments(
    text: &str,
    language: &str,
    blocks: &[crate::services::text_processor::TextBlock],
    use_perplexity: bool,
    use_stylometry: bool,
) -> Vec<SegmentResponse> {
    build_segments_with_profile(
        text,
        language,
        blocks,
        use_perplexity,
        use_stylometry,
        None,
    )
}

/// Build segments from text blocks with optional document profile tuning
pub fn build_segments_with_profile(
    text: &str,
    language: &str,
    blocks: &[crate::services::text_processor::TextBlock],
    use_perplexity: bool,
    use_stylometry: bool,
    doc_profile: Option<&DocumentProfile>,
) -> Vec<SegmentResponse> {
    blocks
        .iter()
        .filter(|b| b.need_detect)
        .enumerate()
        .map(|(idx, block)| {
            make_segment_with_profile(
                idx as i32,
                language,
                block.start,
                block.end,
                &text[block.start as usize..block.end as usize],
                use_perplexity,
                use_stylometry,
                doc_profile,
            )
        })
        .collect()
}

/// Create a segment response with heuristic scoring
pub fn make_segment(
    chunk_id: i32,
    language: &str,
    start: i32,
    end: i32,
    text: &str,
    use_perplexity: bool,
    use_stylometry: bool,
) -> SegmentResponse {
    make_segment_with_profile(
        chunk_id,
        language,
        start,
        end,
        text,
        use_perplexity,
        use_stylometry,
        None,
    )
}

/// Create a segment response with heuristic scoring and optional profile tuning
pub fn make_segment_with_profile(
    chunk_id: i32,
    language: &str,
    start: i32,
    end: i32,
    text: &str,
    use_perplexity: bool,
    use_stylometry: bool,
    doc_profile: Option<&DocumentProfile>,
) -> SegmentResponse {
    let (stylometry, ngram_repeat_rate) = if use_stylometry {
        let metrics = compute_stylometry(text);
        let ngram_repeat_rate = metrics.ngram_repeat_rate.unwrap_or(0.0);
        (
            SignalStylometry {
                ttr: metrics.ttr,
                avg_sentence_len: metrics.avg_sentence_len,
                function_word_ratio: metrics.function_word_ratio,
                repeat_ratio: metrics.repeat_ratio,
                punctuation_ratio: metrics.punctuation_ratio,
            },
            ngram_repeat_rate,
        )
    } else {
        (SignalStylometry::default(), 0.0)
    };

    let ppl = if use_perplexity {
        Some(estimate_perplexity(text))
    } else {
        None
    };
    
    let perplexity = SignalPerplexity {
        ppl,
        z: None,
    };

    // Calculate AI probability using continuous soft-threshold algorithm
    let is_cjk = is_cjk_language(language);
    let (raw_probability, explanations) =
        score_segment_continuous(&stylometry, ngram_repeat_rate, ppl, text, doc_profile, is_cjk);

    // Legacy confidence formula: 0.55 + min(0.35, len(text)/1800), capped at 0.95
    let text_len = text.chars().count() as f64;
    let confidence = (0.55 + (text_len / 1800.0).min(0.35)).min(0.95);
    let uncertainty = (1.0 - confidence).clamp(0.05, 0.9);

    SegmentResponse {
        chunk_id,
        language: language.to_string(),
        offsets: SegmentOffsets { start, end },
        raw_probability,
        confidence,
        uncertainty,
        decision: "pass".to_string(),
        signals: SegmentSignals {
            llm_judgment: SignalLLMJudgment::default(),
            perplexity,
            stylometry,
        },
        explanations,
    }
}

// ============================================================================
// Continuous scoring algorithm v2
// Uses logit-space accumulation with soft thresholds
// ============================================================================

/// Score segment using continuous soft-threshold algorithm
/// Returns (raw_probability, explanations)
fn score_segment_continuous(
    stylometry: &SignalStylometry,
    ngram_repeat_rate: f64,
    ppl: Option<f64>,
    text: &str,
    doc_profile: Option<&DocumentProfile>,
    is_cjk: bool,
) -> (f64, Vec<String>) {
    let mut explanations: Vec<String> = Vec::new();

    // Start in logit space (0.5 probability = 0 logit)
    let mut logit: f64 = 0.0;

    let ttr = stylometry.ttr;
    let rep = stylometry.repeat_ratio.unwrap_or(0.0);
    let ngram = ngram_repeat_rate;
    let avg_len = stylometry.avg_sentence_len;
    let academic_strength = profile_strength(doc_profile);
    // CJK stylometry is character-based; use gentler thresholds/weights to avoid AI bias.
    let (ttr_low_center, ttr_low_k, ttr_high_center, ttr_high_k) = if is_cjk {
        (0.46, 0.08, 0.70, 0.06)
    } else {
        (0.58, 0.08, 0.78, 0.06)
    };
    let (rep_center, rep_k) = if is_cjk { (0.26, 0.07) } else { (0.18, 0.06) };
    let (ngram_center, ngram_k) = if is_cjk { (0.14, 0.05) } else { (0.10, 0.04) };
    let (len_short_center, len_short_k, len_long_center, len_long_k) = if is_cjk {
        (22.0, 8.0, 90.0, 22.0)
    } else {
        (35.0, 10.0, 120.0, 25.0)
    };
    let (ppl_low_center, ppl_low_k, ppl_high_center, ppl_high_k) = if is_cjk {
        (75.0, 18.0, 180.0, 28.0)
    } else {
        (85.0, 20.0, 200.0, 30.0)
    };
    let ttr_low_weight = (1.0 - 0.30 * academic_strength) * if is_cjk { 0.85 } else { 1.0 };
    let repeat_weight = (1.0 - 0.40 * academic_strength) * if is_cjk { 0.75 } else { 1.0 };
    let ngram_weight = (1.0 - 0.40 * academic_strength) * if is_cjk { 0.75 } else { 1.0 };
    let ppl_weight = (1.0 - 0.40 * academic_strength) * if is_cjk { 0.85 } else { 1.0 };
    let ai_anchor_weight = (1.0 - 0.35 * academic_strength) * if is_cjk { 0.9 } else { 1.0 };
    let len_weight = if is_cjk { 0.85 } else { 1.0 };

    // ========================================================================
    // Feature contributions using soft thresholds
    // Each contribution is continuous, not discrete
    // ========================================================================

    // 1. TTR (Type-Token Ratio) - lexical diversity
    // Low TTR suggests AI (more template-like)
    // sigmoid(ttr; center=0.58, k=0.08) gives smooth transition
    let ttr_low_contrib = sigmoid(ttr, ttr_low_center, ttr_low_k) * 1.2 * ttr_low_weight;  // max +1.2 logit
    let ttr_high_contrib = sigmoid_inv(ttr, ttr_high_center, ttr_high_k) * (-0.9);  // max -0.9 logit
    let ttr_contrib = ttr_low_contrib + ttr_high_contrib;
    logit += ttr_contrib;
    if ttr_contrib.abs() > 0.3 {
        explanations.push(format!("ttr={:.3} contrib={:.2}", ttr, ttr_contrib));
    }

    // 2. Repeat ratio - word repetition
    // High repeat suggests AI
    let rep_contrib = sigmoid_inv(rep, rep_center, rep_k) * 1.0 * repeat_weight;  // max +1.0 logit
    logit += rep_contrib;
    if rep_contrib > 0.3 {
        explanations.push(format!("repeat={:.3} contrib={:.2}", rep, rep_contrib));
    }

    // 3. N-gram repeat rate
    // High ngram repeat suggests AI
    let ngram_contrib = sigmoid_inv(ngram, ngram_center, ngram_k) * 1.1 * ngram_weight;  // max +1.1 logit
    logit += ngram_contrib;
    if ngram_contrib > 0.3 {
        explanations.push(format!("ngram={:.3} contrib={:.2}", ngram, ngram_contrib));
    }

    // 4. Average sentence length - U-shaped penalty
    // Very short or very long sentences can indicate AI
    // Optimal range: 40-100 chars
    let len_short_penalty = sigmoid(avg_len, len_short_center, len_short_k) * 0.3 * len_weight;  // penalty for short
    let len_long_penalty = sigmoid_inv(avg_len, len_long_center, len_long_k) * 0.4 * len_weight;  // penalty for long
    let len_contrib = len_short_penalty + len_long_penalty;
    logit += len_contrib;
    if len_contrib.abs() > 0.15 {
        explanations.push(format!("avg_len={:.1} contrib={:.2}", avg_len, len_contrib));
    }

    // 5. Perplexity (if available)
    // Low perplexity suggests AI (more predictable)
    if let Some(ppl_val) = ppl {
        // sigmoid centered at 100, with smooth transition
        let ppl_low_contrib = sigmoid(ppl_val, ppl_low_center, ppl_low_k) * 1.0 * ppl_weight;  // max +1.0 for low ppl
        let ppl_high_contrib = sigmoid_inv(ppl_val, ppl_high_center, ppl_high_k) * (-0.6);  // max -0.6 for high ppl
        let ppl_contrib = ppl_low_contrib + ppl_high_contrib;
        logit += ppl_contrib;
        if ppl_contrib.abs() > 0.2 {
            explanations.push(format!("ppl={:.1} contrib={:.2}", ppl_val, ppl_contrib));
        }
    }

    // ========================================================================
    // Anchor contributions (strong signals, but still continuous)
    // ========================================================================

    // Strong AI anchor: low ttr + low ppl + high repeat
    if let Some(ppl_val) = ppl {
        let anchor_strength =
            sigmoid(ttr, 0.55, 0.05) *           // low ttr
            sigmoid(ppl_val, 90.0, 15.0) *       // low ppl
            (sigmoid_inv(rep, 0.15, 0.04) + sigmoid_inv(ngram, 0.10, 0.03)) / 2.0;  // high repeat

        if anchor_strength > 0.3 {
            let anchor_contrib = anchor_strength * 1.5 * ai_anchor_weight;  // strong positive contribution
            logit += anchor_contrib;
            explanations.push(format!("ai_anchor strength={:.2}", anchor_strength));
        }
    }

    // Strong human anchor: high ttr + high ppl + low repeat + good sentence length
    if let Some(ppl_val) = ppl {
        let human_strength =
            sigmoid_inv(ttr, 0.70, 0.05) *       // high ttr
            sigmoid_inv(ppl_val, 170.0, 25.0) *  // high ppl
            sigmoid(rep, 0.15, 0.04) *           // low repeat
            sigmoid_inv(avg_len, 25.0, 8.0);     // reasonable sentence length

        if human_strength > 0.3 {
            let human_contrib = human_strength * (-1.2);  // strong negative contribution
            logit += human_contrib;
            explanations.push(format!("human_anchor strength={:.2}", human_strength));
        }
    }

    if academic_strength > 0.0 {
        let anchor_strength = academic_anchor_strength(text) * academic_strength;
        if anchor_strength > 0.0 {
            let anchor_contrib = -0.45 * anchor_strength;
            logit += anchor_contrib;
            explanations.push(format!("academic_anchor strength={:.2}", anchor_strength));
        }
    }

    // ========================================================================
    // Convert back to probability and apply deterministic noise
    // ========================================================================

    let mut prob = from_logit(logit);

    // Apply small deterministic noise in critical range [0.35, 0.75]
    // This breaks up quantization without affecting reproducibility
    if prob > 0.35 && prob < 0.75 {
        let noise = deterministic_noise(text, 42) * 0.02;  // ±1% max
        prob = (prob + noise).clamp(0.02, 0.98);
    }

    (prob.clamp(0.02, 0.98), explanations)
}

/// Legacy scoring function - kept for reference but not used
#[allow(dead_code)]
fn score_segment_legacy(
    stylometry: &SignalStylometry,
    ngram_repeat_rate: f64,
    ppl: Option<f64>,
) -> (f64, Vec<String>) {
    let mut score: f64 = 0.5;
    let mut explanations: Vec<String> = Vec::new();

    let ttr = stylometry.ttr;
    let rep = stylometry.repeat_ratio.unwrap_or(0.0);
    let ngram = ngram_repeat_rate;
    let avg_len = stylometry.avg_sentence_len;

    if (ttr < 0.55) && ppl.is_some_and(|p| p < 90.0) && (rep > 0.18 || ngram > 0.12) {
        let lift = (0.12 * (0.55 - ttr).max(0.0) / 0.05).min(0.18);
        score = score.max(0.72 + lift);
        explanations.push("anchor: low ttr + low ppl + high repeat/ngram ? raise".to_string());
    }
    if (ttr >= 0.72)
        && (ppl.is_none() || ppl.is_some_and(|p| p >= 180.0))
        && (rep <= 0.15)
        && (avg_len >= 28.0)
    {
        score = score.min(0.38);
        explanations.push("anchor: human-like signals ? cap low".to_string());
    }

    if (ttr < 0.62) || (rep > 0.18) || (ngram > 0.10) {
        score = score.max(0.60);
        explanations.push("anchor: moderate template signals -> floor 0.60".to_string());
    }
    if (ttr > 0.78) && (avg_len > 35.0) && (rep < 0.12) {
        score = score.min(0.40);
        explanations.push("anchor: moderate human signals -> cap 0.40".to_string());
    }

    if ttr < 0.58 {
        score += 0.18;
        explanations.push("low lexical diversity ? +0.18".to_string());
    } else if ttr > 0.80 {
        score -= 0.16;
        explanations.push("very high lexical diversity ? -0.16".to_string());
    } else if ttr > 0.75 {
        score -= 0.10;
    }

    if rep > 0.30 {
        score += 0.16;
        explanations.push("high unigram repeat ? +0.16".to_string());
    } else if rep > 0.20 {
        score += 0.10;
    }

    if ngram > 0.20 {
        score += 0.18;
        explanations.push("high 3-gram repeat ? +0.18".to_string());
    } else if ngram > 0.12 {
        score += 0.10;
    }

    if avg_len > 140.0 {
        score += 0.06;
    } else if avg_len > 100.0 {
        score += 0.03;
    } else if avg_len < 40.0 {
        score -= 0.03;
    }

    if let Some(ppl_val) = ppl {
        if ppl_val < 70.0 {
            score += 0.18;
            explanations.push("very low perplexity ? +0.18".to_string());
        } else if ppl_val < 90.0 {
            score += 0.12;
        } else if ppl_val > 220.0 {
            score -= 0.12;
        } else if ppl_val > 180.0 {
            score -= 0.08;
        }
    }

    (score.clamp(0.02, 0.98), explanations)
}

/// Estimate perplexity (legacy Python heuristic)
pub fn estimate_perplexity(text: &str) -> f64 {
    let re = Regex::new(r"[A-Za-z0-9_]+|[\u{4e00}-\u{9fff}]").unwrap();
    let tokens: Vec<&str> = re.find_iter(text).map(|m| m.as_str()).collect();
    if tokens.is_empty() {
        return 120.0;
    }

    let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in &tokens {
        *freq.entry(*t).or_insert(0) += 1;
    }

    let total = tokens.len() as f64;
    let entropy = -freq
        .values()
        .map(|&c| {
            let p = c as f64 / total;
            p * (p + 1e-12).ln()
        })
        .sum::<f64>();

    let ppl_uni = entropy.exp();
    let ppl_scaled = 20.0 + ((ppl_uni - 1.0) * 22.5).min(280.0);
    let distinct = freq.len() as f64;
    let diversity = distinct / total.max(1.0);
    let base_old = 120.0 - diversity * 60.0 + (text.chars().count() as f64) / 500.0;
    let val = 0.5 * ppl_scaled + 0.5 * base_old;
    let clamped = val.clamp(20.0, 300.0);
    (clamped * 100.0).round() / 100.0
}
