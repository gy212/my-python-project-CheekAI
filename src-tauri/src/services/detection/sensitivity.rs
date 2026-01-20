// Sensitivity utilities
// Sensitivity influences decision thresholds and gating, not raw probabilities.

use crate::models::{DecisionThresholds, SegmentResponse, SignalLLMEvidence};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DetectionSensitivity {
    Low,
    Medium,
    High,
}

impl DetectionSensitivity {
    pub fn from_str(val: &str) -> Self {
        match val.trim().to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            _ => Self::Medium,
        }
    }
}

#[derive(Debug, Clone)]
struct DecisionProfile {
    thresholds: DecisionThresholds,
    review_uncertainty: f64,
    flag_uncertainty: f64,
    content_min: f64,
    human_max: f64,
}

fn decision_profile(sensitivity: DetectionSensitivity) -> DecisionProfile {
    match sensitivity {
        DetectionSensitivity::Low => DecisionProfile {
            thresholds: DecisionThresholds {
                review: 0.72,
                flag: 0.88,
            },
            review_uncertainty: 0.60,
            flag_uncertainty: 0.30,
            content_min: 0.55,
            human_max: 0.35,
        },
        DetectionSensitivity::High => DecisionProfile {
            thresholds: DecisionThresholds {
                review: 0.55,
                flag: 0.75,
            },
            review_uncertainty: 0.62,
            flag_uncertainty: 0.45,
            content_min: 0.0,
            human_max: 0.55,
        },
        DetectionSensitivity::Medium => DecisionProfile {
            thresholds: DecisionThresholds {
                review: 0.65,
                flag: 0.85,
            },
            review_uncertainty: 0.60,
            flag_uncertainty: 0.35,
            content_min: 0.45,
            human_max: 0.45,
        },
    }
}

pub fn decision_thresholds(sensitivity: &str) -> DecisionThresholds {
    decision_profile(DetectionSensitivity::from_str(sensitivity)).thresholds
}

#[derive(Debug, Copy, Clone, Default)]
pub struct EvidenceSummary {
    pub content_strength: f64,
    pub human_strength: f64,
    pub structural_strength: f64,
}

pub fn summarize_evidence(items: &[SignalLLMEvidence]) -> EvidenceSummary {
    let mut summary = EvidenceSummary::default();
    for item in items {
        let id = item.id.as_str();
        let score = item.score;
        match id {
            "low_specificity" | "logical_leaps" => {
                if score > summary.content_strength {
                    summary.content_strength = score.max(0.0);
                }
            }
            "human_detail" | "stylistic_variance" => {
                if score < 0.0 {
                    summary.human_strength = summary.human_strength.max(-score);
                }
            }
            "template_like" | "uniform_structure" | "high_repetition" | "weak_human_trace" => {
                if score > summary.structural_strength {
                    summary.structural_strength = score.max(0.0);
                }
            }
            _ => {}
        }
    }
    summary
}

fn base_decision(prob: f64, thresholds: &DecisionThresholds, margin: f64) -> &'static str {
    if prob < thresholds.review - margin {
        "pass"
    } else if prob < thresholds.flag - margin {
        "review"
    } else {
        "flag"
    }
}

pub fn decide_segment(
    prob: f64,
    uncertainty: f64,
    evidence: &[SignalLLMEvidence],
    sensitivity: &str,
    margin: f64,
) -> String {
    let profile = decision_profile(DetectionSensitivity::from_str(sensitivity));
    let summary = summarize_evidence(evidence);
    let mut decision = base_decision(prob, &profile.thresholds, margin).to_string();

    if decision == "pass" && uncertainty >= profile.review_uncertainty {
        decision = "review".to_string();
    }

    if decision == "flag" {
        if uncertainty > profile.flag_uncertainty {
            decision = "review".to_string();
        } else if profile.content_min > 0.0 && summary.content_strength < profile.content_min {
            decision = "review".to_string();
        } else if summary.human_strength >= profile.human_max {
            decision = "review".to_string();
        }
    }

    decision
}

pub fn apply_segment_decisions(
    segments: &mut [SegmentResponse],
    sensitivity: &str,
    margin: f64,
) {
    for seg in segments.iter_mut() {
        let evidence = seg.signals.llm_judgment.evidence.as_slice();
        seg.decision = decide_segment(seg.raw_probability, seg.uncertainty, evidence, sensitivity, margin);
    }
}

pub fn decide_overall(
    prob: f64,
    overall_uncertainty: f64,
    segments: &[SegmentResponse],
    sensitivity: &str,
    margin: f64,
) -> String {
    let profile = decision_profile(DetectionSensitivity::from_str(sensitivity));
    let mut decision = base_decision(prob, &profile.thresholds, margin).to_string();

    if decision == "pass" && overall_uncertainty >= profile.review_uncertainty {
        decision = "review".to_string();
    }

    if decision == "flag" {
        if overall_uncertainty > profile.flag_uncertainty {
            decision = "review".to_string();
        } else {
            let mut has_gate = false;
            for seg in segments {
                if seg.raw_probability < profile.thresholds.flag - margin {
                    continue;
                }
                if seg.uncertainty > profile.flag_uncertainty {
                    continue;
                }
                let summary = summarize_evidence(&seg.signals.llm_judgment.evidence);
                if profile.content_min > 0.0 && summary.content_strength < profile.content_min {
                    continue;
                }
                if summary.human_strength >= profile.human_max {
                    continue;
                }
                has_gate = true;
                break;
            }
            if !has_gate {
                decision = "review".to_string();
            }
        }
    }

    decision
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SegmentOffsets, SegmentSignals, SignalLLMEvidence};

    #[test]
    fn test_thresholds_order() {
        let low = decision_thresholds("low");
        let mid = decision_thresholds("medium");
        let high = decision_thresholds("high");
        assert!(low.review > mid.review);
        assert!(mid.review > high.review);
    }

    #[test]
    fn test_segment_flag_gate_requires_content() {
        let mut seg = SegmentResponse {
            chunk_id: 0,
            language: "zh".to_string(),
            offsets: SegmentOffsets { start: 0, end: 10 },
            raw_probability: 0.9,
            confidence: 0.8,
            uncertainty: 0.2,
            decision: "".to_string(),
            signals: SegmentSignals::default(),
            explanations: vec![],
        };
        seg.signals.llm_judgment.evidence = vec![SignalLLMEvidence {
            id: "template_like".to_string(),
            score: 0.8,
            evidence: "模板".to_string(),
        }];
        let decision = decide_segment(seg.raw_probability, seg.uncertainty, &seg.signals.llm_judgment.evidence, "medium", 0.03);
        assert_eq!(decision, "review");
    }
}
