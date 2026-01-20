// LLM Analyzer
// Handles LLM-based text analysis for AI detection
// Supports:
// - Batch paragraph analysis via GLM
// - Sentence-level analysis via DeepSeek with length filtering

use crate::models::{DocumentProfile, SegmentResponse, SignalLLMEvidence, SignalLLMJudgment};
use crate::services::providers::{get_api_key, parse_provider, ProviderClient, OPENAI_DEFAULT_MODEL};
use crate::services::ConfigStore;
use crate::services::text_processor::{compute_stylometry, estimate_tokens, TextBlock};
use serde::Deserialize;
use std::cmp::Ordering;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{info, warn};

use super::segment_builder::{build_segments, build_segments_with_profile, make_segment, make_segment_with_profile};
use super::sensitivity::summarize_evidence;
use super::subject_catalog::{is_academic_profile, profile_validity, validate_document_profile, ProfileValidity};

/// Sentence length thresholds (Unicode scalar count, not UTF-8 byte length)
const SENTENCE_MIN_LENGTH: usize = 10;  // Skip sentences shorter than this
const SENTENCE_LLM_THRESHOLD: usize = 50;  // Send to LLM if >= this length
const SENTENCE_REASONER_THRESHOLD: usize = 300; // Use DeepSeek reasoner for very long segments
const DEEPSEEK_SENTENCE_MAX_CONCURRENCY: usize = 10;
const DEEPSEEK_SENTENCE_MAX_ATTEMPTS: usize = 3; // initial + retries
const DEEPSEEK_SENTENCE_TIMEOUT_SECS: u64 = 60;

/// LLM output budget.
///
/// Some gateways/models (notably Gemini) may spend a large portion of the budget on internal
/// reasoning/thoughts, which can cause empty content unless `max_tokens` is set sufficiently high.
const LLM_MAX_TOKENS: i32 = 8192;

const LLM_SEGMENT_MAX_CONCURRENCY: usize = 10;
const DOC_PROFILE_MAX_TOKENS: i32 = 6000;
const CONTEXT_MAX_CHARS: usize = 600;

static LLM_SEGMENT_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn llm_segment_semaphore() -> &'static Arc<Semaphore> {
    LLM_SEGMENT_SEMAPHORE.get_or_init(|| Arc::new(Semaphore::new(LLM_SEGMENT_MAX_CONCURRENCY)))
}

fn detection_system_prompt() -> &'static str {
    DETECTION_SYSTEM_PROMPT
}

/// System prompt for single segment AI detection
const DETECTION_SYSTEM_PROMPT: &str = r#"你是一个专业的AI文本检测专家。你需要判断给定的文本是否由AI生成。
请分析文本的以下特征：
1. 语言流畅度和自然程度
2. 是否存在AI生成文本的典型特征（如过度正式、缺乏个人风格、重复模板等）
3. 内容的逻辑性和连贯性
若文档概况显示为学术写作，模板化/结构均匀/重复等可能是写作常态，不能单独作为强证据，应提高 uncertainty 并结合反证。

请以JSON格式返回结果，包含以下字段：
- probability: 0.000-1.000之间的三位小数，表示文本是AI生成的概率
- confidence: 0.000-1.000之间的三位小数，表示你对判断的置信度
- uncertainty: 0.000-1.000之间的三位小数，表示不确定性（越高越不确定）
- signals: 证据数组，每项包含 {id, score, evidence}

signals.id 只能从以下列表选择：
- template_like: 模板化表达/固定句式
- low_specificity: 抽象泛化、缺少可验证细节
- uniform_structure: 段落节奏/结构过于均匀
- high_repetition: 重复/近似句式/高 n-gram
- weak_human_trace: 缺少个人经历、过程、时间地点等痕迹
- logical_leaps: 论证跳跃、过度总结、前后衔接薄弱
- human_detail: 反证，具体经历/细节/可验证信息
- stylistic_variance: 反证，风格波动/个性化表达

signals.score: -1.000到1.000，正数=AI证据，负数=人类反证，绝对值表示强度。
signals数量控制在3-6条，尽量给出最关键的证据。

重要：probability/confidence/uncertainty必须是三位小数。
只返回JSON，不要有其他文字。"#;

/// System prompt for batch paragraph AI detection (GLM)
const BATCH_DETECTION_SYSTEM_PROMPT: &str = r#"你是一个专业的AI文本检测专家。你需要判断给定的多个文本段落是否由AI生成。

请分析每个段落的以下特征：
1. 语言流畅度和自然程度
2. 是否存在AI生成文本的典型特征（如过度正式、缺乏个人风格、重复模板等）
3. 内容的逻辑性和连贯性

请以JSON格式返回结果，包含一个segments数组，每个元素包含：
- chunk_id: 段落编号（从0开始）
- probability: 0.000-1.000之间的三位小数，表示该段落是AI生成的概率
- confidence: 0.000-1.000之间的三位小数，表示你对该判断的置信度
- uncertainty: 0.000-1.000之间的三位小数，表示不确定性（越高越不确定）
- signals: 证据数组，每项包含 {id, score, evidence}

signals.id 只能从以下列表选择：
- template_like: 模板化表达/固定句式
- low_specificity: 抽象泛化、缺少可验证细节
- uniform_structure: 段落节奏/结构过于均匀
- high_repetition: 重复/近似句式/高 n-gram
- weak_human_trace: 缺少个人经历、过程、时间地点等痕迹
- logical_leaps: 论证跳跃、过度总结、前后衔接薄弱
- human_detail: 反证，具体经历/细节/可验证信息
- stylistic_variance: 反证，风格波动/个性化表达

signals.score: -1.000到1.000，正数=AI证据，负数=人类反证，绝对值表示强度。
signals数量控制在3-6条，尽量给出最关键的证据。

重要：probability/confidence/uncertainty必须是三位小数。
只返回JSON，不要有其他文字。

示例格式：
{"segments": [{"chunk_id": 0, "probability": 0.723, "confidence": 0.856, "uncertainty": 0.210, "signals": [{"id": "template_like", "score": 0.62, "evidence": "..."}]}, {"chunk_id": 1, "probability": 0.312, "confidence": 0.945, "uncertainty": 0.180, "signals": [{"id": "human_detail", "score": -0.55, "evidence": "..."}]}]}
"#;

/// System prompt for document-level profile (discipline + summary)
const DOCUMENT_PROFILE_SYSTEM_PROMPT: &str = r#"你是学术写作分析助手。请根据全文内容输出文档概况，用于后续段落判定的语境参考。

请只返回 JSON，字段如下：
- category: 学科门类（必须从教育部学科目录门类中选择：哲学、经济学、法学、教育学、文学、历史学、理学、工学、农学、医学、军事学、管理学、艺术学、交叉学科）
- discipline: 一级学科（教育部学科目录标准名称，无法判断请填“交叉学科”）
- subfield: 二级学科/研究方向（可选）
- paperType: 论文类型（不限，短词即可，如 论文/综述/实验报告/课程论文/技术报告/说明文 等）
- summary: 一句话摘要（不超过40字）
- conventions: 写作约定/文体特征数组（3-6条，短句即可）

注意：category 不能用“学术论文/论文”等泛化类型。
只返回 JSON，不要有其他文字。"#;

/// LLM judgment response for single segment
#[derive(Debug, Deserialize, Default)]
struct LLMJudgment {
    #[serde(default)]
    probability: f64,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    uncertainty: Option<f64>,
    #[serde(default)]
    signals: Vec<SignalLLMEvidence>,
    #[serde(default)]
    reasoning: Option<String>,
}

/// LLM judgment response for batch segments
#[derive(Debug, Deserialize, Default)]
struct BatchLLMJudgment {
    #[serde(default)]
    segments: Vec<SegmentJudgment>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct SegmentJudgment {
    #[serde(default)]
    chunk_id: i32,
    #[serde(default)]
    probability: f64,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    uncertainty: Option<f64>,
    #[serde(default)]
    signals: Vec<SignalLLMEvidence>,
}

#[derive(Debug, Clone, Default)]
struct SegmentContext {
    prev: Option<String>,
    next: Option<String>,
}

fn default_confidence() -> f64 {
    0.6
}

fn build_provider_client_with_proxy() -> ProviderClient {
    let config = ConfigStore::default_config_dir()
        .map(ConfigStore::new)
        .and_then(|store| store.load().ok());

    let proxy_url = config
        .as_ref()
        .and_then(|c| c.proxy.as_ref())
        .filter(|p| p.enabled)
        .and_then(|p| p.https.as_deref().or(p.http.as_deref()))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    match proxy_url.as_deref() {
        Some(p) => ProviderClient::with_proxy(p).unwrap_or_else(|_| ProviderClient::new()),
        None => ProviderClient::new(),
    }
}

fn resolve_provider_spec(provider: Option<&str>) -> Option<(String, String, String)> {
    let selected = provider.and_then(|p| {
        let spec = parse_provider(p);
        let default_model = match spec.name.as_str() {
            "openai" => OPENAI_DEFAULT_MODEL.to_string(),
            "gemini" => "gemini-3-pro-preview".to_string(),
            "glm" => "glm-4-flash".to_string(),
            "deepseek" => "deepseek-chat".to_string(),
            "anthropic" | "claude" => "claude-sonnet-4-20250514".to_string(),
            _ => spec.model.clone(),
        };
        let model = if spec.model.trim().is_empty() { default_model } else { spec.model };
        get_api_key(&spec.name).map(|k| (spec.name, model, k))
    });

    selected.or_else(|| {
        if let Some(key) = get_api_key("openai") {
            Some(("openai".to_string(), OPENAI_DEFAULT_MODEL.to_string(), key))
        } else if let Some(key) = get_api_key("gemini") {
            Some(("gemini".to_string(), "gemini-3-pro-preview".to_string(), key))
        } else if let Some(key) = get_api_key("glm") {
            Some(("glm".to_string(), "glm-4-flash".to_string(), key))
        } else if let Some(key) = get_api_key("deepseek") {
            Some(("deepseek".to_string(), "deepseek-chat".to_string(), key))
        } else if let Some(key) = get_api_key("anthropic") {
            Some(("anthropic".to_string(), "claude-sonnet-4-20250514".to_string(), key))
        } else {
            None
        }
    })
}

fn resolve_custom_url(provider_name: &str) -> Option<String> {
    ConfigStore::default_config_dir()
        .map(ConfigStore::new)
        .and_then(|store| store.get_provider_url(provider_name).ok().flatten())
        .or_else(|| {
            // Support common aliases stored in config.
            if provider_name == "anthropic" {
                ConfigStore::default_config_dir()
                    .map(ConfigStore::new)
                    .and_then(|store| store.get_provider_url("claude").ok().flatten())
            } else if provider_name == "claude" {
                ConfigStore::default_config_dir()
                    .map(ConfigStore::new)
                    .and_then(|store| store.get_provider_url("anthropic").ok().flatten())
            } else {
                None
            }
        })
}

fn truncate_context(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut out = String::new();
    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => out.push(ch),
            None => break,
        }
    }
    out
}

fn build_segment_context(block: &TextBlock, all_blocks: &[TextBlock]) -> SegmentContext {
    let prev_index = block.index - 1;
    let next_index = block.index + 1;

    let prev = all_blocks
        .get(prev_index as usize)
        .filter(|b| b.index == prev_index)
        .or_else(|| all_blocks.iter().find(|b| b.index == prev_index))
        .map(|b| truncate_context(&b.text, CONTEXT_MAX_CHARS));

    let next = all_blocks
        .get(next_index as usize)
        .filter(|b| b.index == next_index)
        .or_else(|| all_blocks.iter().find(|b| b.index == next_index))
        .map(|b| truncate_context(&b.text, CONTEXT_MAX_CHARS));

    SegmentContext { prev, next }
}

fn build_document_profile_input(text: &str, blocks: &[TextBlock]) -> String {
    if estimate_tokens(text) <= DOC_PROFILE_MAX_TOKENS {
        return text.to_string();
    }

    let mut selected_indices: Vec<i32> = Vec::new();
    let mut push_index = |index: i32| {
        if !selected_indices.contains(&index) {
            selected_indices.push(index);
        }
    };

    let total = blocks.len();
    for block in blocks.iter().take(3) {
        push_index(block.index);
    }
    if total > 6 {
        let step = ((total - 5) / 4).max(1);
        for block in blocks.iter().skip(3).take(total.saturating_sub(5)).step_by(step) {
            push_index(block.index);
        }
    }
    for block in blocks.iter().rev().take(2) {
        push_index(block.index);
    }

    selected_indices.sort();

    let mut parts: Vec<String> = Vec::new();
    let mut tokens: i32 = 0;
    for index in selected_indices {
        let block = match blocks.iter().find(|b| b.index == index) {
            Some(block) => block,
            None => continue,
        };
        let block_tokens = estimate_tokens(&block.text);
        if tokens + block_tokens > DOC_PROFILE_MAX_TOKENS {
            break;
        }
        tokens += block_tokens;
        parts.push(block.text.clone());
    }

    if parts.is_empty() {
        let truncated = truncate_context(text, 4000);
        return truncated;
    }

    parts.join("\n\n")
}

const EDU_DISCIPLINE_DOMAINS: [&str; 14] = [
    "哲学",
    "经济学",
    "法学",
    "教育学",
    "文学",
    "历史学",
    "理学",
    "工学",
    "农学",
    "医学",
    "军事学",
    "管理学",
    "艺术学",
    "交叉学科",
];

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn infer_domain_from_discipline(discipline: Option<&str>) -> Option<&'static str> {
    let discipline = discipline?.trim();
    if discipline.is_empty() {
        return None;
    }
    let lower = discipline.to_lowercase();
    if discipline.contains("哲学") || lower.contains("philosophy") {
        return Some("哲学");
    }
    if discipline.contains("经济")
        || discipline.contains("金融")
        || discipline.contains("会计")
        || discipline.contains("财政")
        || lower.contains("economics")
        || lower.contains("finance")
    {
        return Some("经济学");
    }
    if discipline.contains("法学")
        || discipline.contains("法律")
        || discipline.contains("司法")
        || discipline.contains("公安")
        || discipline.contains("政治")
        || discipline.contains("社会")
        || lower.contains("law")
        || lower.contains("politic")
        || lower.contains("sociology")
    {
        return Some("法学");
    }
    if discipline.contains("教育") || lower.contains("education") {
        return Some("教育学");
    }
    if discipline.contains("文学")
        || discipline.contains("语言")
        || discipline.contains("新闻")
        || discipline.contains("传播")
        || lower.contains("literature")
        || lower.contains("linguistics")
        || lower.contains("journalism")
        || lower.contains("communication")
    {
        return Some("文学");
    }
    if discipline.contains("历史") || discipline.contains("考古") || lower.contains("history") {
        return Some("历史学");
    }
    if discipline.contains("数学")
        || discipline.contains("物理")
        || discipline.contains("化学")
        || discipline.contains("生物")
        || discipline.contains("地理")
        || discipline.contains("天文")
        || discipline.contains("地球")
        || lower.contains("science")
        || lower.contains("mathematics")
        || lower.contains("physics")
        || lower.contains("chemistry")
        || lower.contains("biology")
        || lower.contains("geology")
        || lower.contains("astronomy")
    {
        return Some("理学");
    }
    if discipline.contains("工程")
        || discipline.contains("工学")
        || discipline.contains("计算机")
        || discipline.contains("信息")
        || discipline.contains("电子")
        || discipline.contains("通信")
        || discipline.contains("材料")
        || discipline.contains("机械")
        || discipline.contains("土木")
        || discipline.contains("建筑")
        || discipline.contains("环境")
        || discipline.contains("化工")
        || discipline.contains("软件")
        || lower.contains("engineering")
        || lower.contains("computer")
        || lower.contains("information")
        || lower.contains("software")
    {
        return Some("工学");
    }
    if discipline.contains("农")
        || discipline.contains("林")
        || discipline.contains("畜")
        || discipline.contains("兽")
        || discipline.contains("水产")
        || lower.contains("agriculture")
        || lower.contains("forestry")
        || lower.contains("veterinary")
    {
        return Some("农学");
    }
    if discipline.contains("医学")
        || discipline.contains("临床")
        || discipline.contains("护理")
        || discipline.contains("药学")
        || discipline.contains("口腔")
        || discipline.contains("中医")
        || discipline.contains("公共卫生")
        || lower.contains("medicine")
        || lower.contains("clinical")
        || lower.contains("nursing")
        || lower.contains("pharmacy")
    {
        return Some("医学");
    }
    if discipline.contains("军事") || lower.contains("military") || lower.contains("defense") {
        return Some("军事学");
    }
    if discipline.contains("管理")
        || discipline.contains("工商")
        || discipline.contains("公共管理")
        || discipline.contains("图书")
        || discipline.contains("档案")
        || discipline.contains("信息管理")
        || lower.contains("management")
        || lower.contains("business")
        || lower.contains("administration")
    {
        return Some("管理学");
    }
    if discipline.contains("艺术")
        || discipline.contains("美术")
        || discipline.contains("音乐")
        || discipline.contains("戏剧")
        || discipline.contains("舞蹈")
        || discipline.contains("设计")
        || lower.contains("art")
        || lower.contains("design")
        || lower.contains("music")
        || lower.contains("drama")
    {
        return Some("艺术学");
    }
    if discipline.contains("交叉") || lower.contains("interdisciplinary") {
        return Some("交叉学科");
    }
    None
}

fn match_domain(value: &str, discipline: Option<&str>) -> Option<&'static str> {
    for domain in EDU_DISCIPLINE_DOMAINS.iter() {
        if value.contains(domain) {
            return Some(*domain);
        }
    }

    if value.contains("理科") {
        return Some("理学");
    }
    if value.contains("工科") {
        return Some("工学");
    }
    if value.contains("医科") {
        return Some("医学");
    }
    if value.contains("农科") {
        return Some("农学");
    }
    if value.contains("商科") || value.contains("经管") {
        if let Some(domain) = infer_domain_from_discipline(discipline) {
            if domain == "经济学" || domain == "管理学" {
                return Some(domain);
            }
        }
        return Some("管理学");
    }
    if value.contains("文科") {
        return Some("文学");
    }

    infer_domain_from_discipline(Some(value))
}

fn normalize_domain(value: &str, discipline: Option<&str>) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return infer_domain_from_discipline(discipline)
            .unwrap_or("交叉学科")
            .to_string();
    }
    if let Some(domain) = match_domain(trimmed, discipline) {
        return domain.to_string();
    }
    if let Some(domain) = infer_domain_from_discipline(discipline) {
        return domain.to_string();
    }
    "交叉学科".to_string()
}

fn looks_like_paper_type(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    trimmed.contains("论文")
        || trimmed.contains("综述")
        || trimmed.contains("实验")
        || trimmed.contains("报告")
        || trimmed.contains("作业")
        || trimmed.contains("课程")
        || trimmed.contains("说明文")
        || trimmed.contains("调查")
        || trimmed.contains("调研")
        || trimmed.contains("毕业设计")
        || lower.contains("thesis")
        || lower.contains("paper")
        || lower.contains("report")
        || lower.contains("survey")
}

fn format_doc_profile(profile: &DocumentProfile) -> String {
    let mut s = String::new();
    s.push_str("文档概况:\n");
    s.push_str(&format!("- 学科门类: {}\n", profile.category));
    if let Some(discipline) = profile.discipline.as_ref().filter(|f| !f.trim().is_empty()) {
        s.push_str(&format!("- 一级学科: {}\n", discipline.trim()));
    }
    if let Some(subfield) = profile.subfield.as_ref().filter(|f| !f.trim().is_empty()) {
        s.push_str(&format!("- 二级学科/方向: {}\n", subfield.trim()));
    }
    if let Some(paper_type) = profile.paper_type.as_ref().filter(|f| !f.trim().is_empty()) {
        s.push_str(&format!("- 论文类型: {}\n", paper_type.trim()));
    }
    s.push_str(&format!("- 一句话摘要: {}\n", profile.summary));
    if !profile.conventions.is_empty() {
        s.push_str("- 写作约定:\n");
        for item in profile.conventions.iter().take(6) {
            if !item.trim().is_empty() {
                s.push_str(&format!("  - {}\n", item.trim()));
            }
        }
    }
    s
}

fn normalize_llm_evidence(
    items: Vec<SignalLLMEvidence>,
    profile: Option<&DocumentProfile>,
) -> Vec<SignalLLMEvidence> {
    let mut normalized: Vec<SignalLLMEvidence> = items
        .into_iter()
        .map(|mut item| {
            item.id = item.id.trim().to_lowercase();
            item.score = item.score.clamp(-1.0, 1.0);
            item.evidence = item.evidence.trim().to_string();
            item
        })
        .filter(|item| {
            !item.id.is_empty()
                && !item.evidence.is_empty()
                && evidence_weight(&item.id, profile) > 0.0
        })
        .collect();

    normalized.sort_by(|a, b| {
        b.score
            .abs()
            .partial_cmp(&a.score.abs())
            .unwrap_or(Ordering::Equal)
    });

    normalized
}

fn evidence_weight(id: &str, profile: Option<&DocumentProfile>) -> f64 {
    let mut weight = match id {
        "template_like" => 1.0,
        "low_specificity" => 0.9,
        "uniform_structure" => 0.8,
        "high_repetition" => 0.9,
        "weak_human_trace" => 0.7,
        "logical_leaps" => 0.7,
        "human_detail" => 1.0,
        "stylistic_variance" => 0.7,
        _ => 0.0,
    };

    if let Some(profile) = profile {
        if is_academic_profile(profile) {
            let strength = match profile_validity(profile) {
                ProfileValidity::Valid => 1.0,
                ProfileValidity::Partial => 0.6,
                ProfileValidity::Invalid => 0.0,
            };
            match id {
                "template_like" => weight *= 1.0 - 0.6 * strength,
                "uniform_structure" => weight *= 1.0 - 0.6 * strength,
                "high_repetition" => weight *= 1.0 - 0.4 * strength,
                "weak_human_trace" => weight *= 1.0 - 0.65 * strength,
                "low_specificity" => weight *= 1.0 - 0.25 * strength,
                _ => {}
            }
        }
    }

    weight
}

fn evidence_probability(
    items: &[SignalLLMEvidence],
    profile: Option<&DocumentProfile>,
) -> Option<f64> {
    if items.is_empty() {
        return None;
    }

    let mut score = 0.0;
    let mut used = 0usize;
    for item in items {
        let w = evidence_weight(&item.id, profile);
        if w <= 0.0 {
            continue;
        }
        score += w * item.score;
        used += 1;
    }

    if used == 0 {
        return None;
    }

    let score = score.clamp(-3.0, 3.0);
    Some(1.0 / (1.0 + (-score).exp()))
}

fn blend_llm_with_evidence(llm_prob: f64, evidence_prob: Option<f64>) -> f64 {
    if let Some(p) = evidence_prob {
        let gap = (llm_prob - p).abs().clamp(0.0, 1.0);
        let mix = 0.20 + 0.40 * gap;
        (llm_prob * (1.0 - mix) + p * mix).clamp(0.0, 1.0)
    } else {
        llm_prob
    }
}

fn adjust_llm_confidence(
    llm_prob: f64,
    llm_conf: f64,
    uncertainty: Option<f64>,
    evidence_prob: Option<f64>,
) -> f64 {
    let mut conf = llm_conf.clamp(0.0, 1.0);
    let uncertainty = uncertainty.unwrap_or(0.0).clamp(0.0, 1.0);
    conf *= 1.0 - uncertainty;

    if let Some(p) = evidence_prob {
        let consistency = 1.0 - (llm_prob - p).abs().clamp(0.0, 1.0);
        conf *= 0.6 + 0.4 * consistency;
    }

    conf.clamp(0.0, 1.0)
}

fn fuse_probabilities(
    local_prob: f64,
    local_conf: f64,
    llm_prob: f64,
    llm_conf: f64,
    text_len: usize,
    evidence_prob: Option<f64>,
) -> (f64, f64) {
    let len_factor = (text_len as f64 / 1200.0).clamp(0.0, 1.0);
    let conflict_local = (llm_prob - local_prob).abs();
    let conflict_evidence = evidence_prob.map(|p| (llm_prob - p).abs()).unwrap_or(0.0);
    let conflict = conflict_local.max(conflict_evidence * 0.8);

    let base = 0.27;
    let max_w = 0.62;
    let conflict_penalty = 0.75;

    let mut w = base + 0.45 * llm_conf + 0.15 * len_factor - conflict_penalty * conflict;
    w = w.clamp(0.15, max_w);

    let prob = w * llm_prob + (1.0 - w) * local_prob;
    let conf = (0.55 * llm_conf + 0.45 * local_conf) * (1.0 - 0.35 * conflict);

    (prob.clamp(0.0, 1.0), conf.clamp(0.0, 1.0))
}

fn apply_llm_judgment_to_segment(
    segment: &mut SegmentResponse,
    llm_prob: f64,
    llm_conf: f64,
    llm_uncertainty: Option<f64>,
    llm_evidence: Vec<SignalLLMEvidence>,
    model_label: String,
    text_len: usize,
    doc_profile: Option<&DocumentProfile>,
) {
    let local_prob = segment.raw_probability;
    let local_conf = segment.confidence;

    let evidence = normalize_llm_evidence(llm_evidence, doc_profile);
    let evidence_prob = evidence_probability(&evidence, doc_profile);
    let llm_prob_raw = llm_prob.clamp(0.0, 1.0);
    let llm_conf_raw = llm_conf.clamp(0.0, 1.0);
    let llm_prob_adj = blend_llm_with_evidence(llm_prob_raw, evidence_prob);
    let llm_conf_adj = adjust_llm_confidence(llm_prob_raw, llm_conf_raw, llm_uncertainty, evidence_prob);

    let (fused_prob, fused_conf) = fuse_probabilities(
        local_prob,
        local_conf,
        llm_prob_adj,
        llm_conf_adj,
        text_len,
        evidence_prob,
    );

    let mut uncertainty = llm_uncertainty
        .unwrap_or(1.0 - llm_conf_adj)
        .clamp(0.0, 1.0);
    if let Some(p) = evidence_prob {
        let conflict = (llm_prob_adj - p).abs().clamp(0.0, 1.0);
        uncertainty = (uncertainty + 0.35 * conflict).clamp(0.0, 1.0);
    }
    if evidence.is_empty() {
        uncertainty = uncertainty.max(0.5);
    }
    if let Some(profile) = doc_profile {
        if is_academic_profile(profile) {
            let strength = match profile_validity(profile) {
                ProfileValidity::Valid => 1.0,
                ProfileValidity::Partial => 0.6,
                ProfileValidity::Invalid => 0.0,
            };
            if strength > 0.0 {
                let summary = summarize_evidence(&evidence);
                if summary.structural_strength > 0.45 && summary.content_strength < 0.2 {
                    uncertainty = (uncertainty + 0.12 * strength).clamp(0.0, 1.0);
                    segment.explanations.push("academic_structure_uncertainty_boost".to_string());
                }
            }
        }
    }

    segment.raw_probability = fused_prob;
    segment.confidence = fused_conf;
    segment.uncertainty = uncertainty;
    segment.signals.llm_judgment = SignalLLMJudgment {
        prob: Some(llm_prob_raw),
        models: vec![model_label],
        uncertainty: llm_uncertainty,
        evidence,
    };
}

fn build_segment_user_prompt(
    text: &str,
    context: Option<&SegmentContext>,
    doc_profile: Option<&DocumentProfile>,
) -> String {
    let mut prompt = String::new();
    if let Some(profile) = doc_profile {
        prompt.push_str(&format_doc_profile(profile));
        prompt.push('\n');
    }

    prompt.push_str("上下文（仅供参考，不进行判定）：\n");
    if let Some(ctx) = context {
        if let Some(prev) = ctx.prev.as_ref().filter(|s| !s.trim().is_empty()) {
            prompt.push_str("[上一段]\n");
            prompt.push_str(prev.trim());
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str("[本段]\n");
    prompt.push_str(text.trim());
    prompt.push_str("\n\n");

    if let Some(ctx) = context {
        if let Some(next) = ctx.next.as_ref().filter(|s| !s.trim().is_empty()) {
            prompt.push_str("[下一段]\n");
            prompt.push_str(next.trim());
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str("请只对[本段]输出JSON结果，勿把上下文当作判定对象。");
    prompt
}

fn enrich_llm_call_error(message: &str) -> String {
    if message.contains("API error: 401") {
        format!("{}（请检查 Token/API Key 是否正确或已过期）", message)
    } else {
        message.to_string()
    }
}

fn build_document_profile_user_prompt(text: &str, blocks: &[TextBlock]) -> String {
    let sample = build_document_profile_input(text, blocks);
    format!(
        "请根据以下内容输出文档概况 JSON，并严格对齐教育部学科目录口径。\n\n{}",
        sample
    )
}

async fn call_llm_document_profile_with_url(
    client: &ProviderClient,
    text: &str,
    blocks: &[TextBlock],
    provider_name: &str,
    model: &str,
    api_key: &str,
    custom_url: Option<&str>,
) -> Result<DocumentProfile, String> {
    let user_prompt = build_document_profile_user_prompt(text, blocks);

    let result = if provider_name == "deepseek" {
        client
            .call_deepseek_json_with_url(
                custom_url,
                model,
                api_key,
                DOCUMENT_PROFILE_SYSTEM_PROMPT,
                &user_prompt,
                LLM_MAX_TOKENS,
            )
            .await
    } else if provider_name == "gemini" {
        client
            .call_gemini_with_url(
                custom_url,
                model,
                api_key,
                DOCUMENT_PROFILE_SYSTEM_PROMPT,
                &user_prompt,
                LLM_MAX_TOKENS,
            )
            .await
    } else if provider_name == "openai" {
        let combined = format!("{}\n\n{}", DOCUMENT_PROFILE_SYSTEM_PROMPT, user_prompt);
        client
            .call_openai_responses_with_url(custom_url, model, api_key, &combined)
            .await
    } else if provider_name == "anthropic" || provider_name == "claude" {
        client
            .call_anthropic_with_url(
                custom_url,
                model,
                api_key,
                DOCUMENT_PROFILE_SYSTEM_PROMPT,
                &user_prompt,
                LLM_MAX_TOKENS,
            )
            .await
    } else {
        client
            .call_glm_with_url(
                custom_url,
                model,
                api_key,
                DOCUMENT_PROFILE_SYSTEM_PROMPT,
                &user_prompt,
                LLM_MAX_TOKENS,
                false,
            )
            .await
    };

    match result {
        Ok(chat_result) => {
            let json_str = extract_json(chat_result.content.trim())?;
            let mut profile: DocumentProfile = serde_json::from_str(&json_str)
                .map_err(|e| format!("JSON parse error: {}", e))?;
            let raw_category = profile.category.trim().to_string();
            profile.summary = profile.summary.trim().to_string();
            profile.discipline = normalize_optional_text(profile.discipline);
            profile.subfield = normalize_optional_text(profile.subfield);
            profile.paper_type = normalize_optional_text(profile.paper_type);
            let discipline_hint = profile
                .discipline
                .as_deref()
                .or(profile.subfield.as_deref());
            profile.category = normalize_domain(&raw_category, discipline_hint);
            if profile.paper_type.is_none() && looks_like_paper_type(&raw_category) {
                profile.paper_type = Some(raw_category);
            }
            profile.conventions = profile
                .conventions
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if profile.summary.is_empty() {
                return Err("Document profile missing summary".to_string());
            }
            validate_document_profile(&mut profile);
            Ok(profile)
        }
        Err(e) => {
            let msg = e.to_string();
            Err(format!("Document profile call failed: {}", enrich_llm_call_error(&msg)))
        }
    }
}

async fn call_llm_for_segment_with_url(
    client: &ProviderClient,
    text: &str,
    provider_name: &str,
    model: &str,
    api_key: &str,
    custom_url: Option<&str>,
    doc_profile: Option<&DocumentProfile>,
    context: Option<&SegmentContext>,
) -> Result<LLMJudgment, String> {
    let system_prompt = detection_system_prompt();
    let user_prompt = build_segment_user_prompt(text, context, doc_profile);

    let result = if provider_name == "deepseek" {
        // Use call_deepseek_json since prompt contains 'json'
        client
            .call_deepseek_json_with_url(
                custom_url,
                model,
                api_key,
                &system_prompt,
                &user_prompt,
                LLM_MAX_TOKENS,
            )
            .await
    } else if provider_name == "gemini" {
        client
            .call_gemini_with_url(
                custom_url,
                model,
                api_key,
                &system_prompt,
                &user_prompt,
                LLM_MAX_TOKENS,
            )
            .await
    } else if provider_name == "openai" {
        let combined = format!("{}\n\n{}", system_prompt, user_prompt);
        client
            .call_openai_responses_with_url(custom_url, model, api_key, &combined)
            .await
    } else if provider_name == "anthropic" || provider_name == "claude" {
        client
            .call_anthropic_with_url(
                custom_url,
                model,
                api_key,
                &system_prompt,
                &user_prompt,
                LLM_MAX_TOKENS,
            )
            .await
    } else {
        client
            .call_glm_with_url(
                custom_url,
                model,
                api_key,
                &system_prompt,
                &user_prompt,
                LLM_MAX_TOKENS,
                false,
            )
            .await
    };

    match result {
        Ok(chat_result) => parse_single_judgment(&chat_result.content),
        Err(e) => {
            let msg = e.to_string();
            Err(format!("LLM call failed: {}", enrich_llm_call_error(&msg)))
        }
    }
}

async fn call_deepseek_for_sentence(
    client: &ProviderClient,
    text: &str,
    api_key: &str,
    model: &str,
    chunk_id: i32,
    start: i32,
    end: i32,
    doc_profile: Option<&DocumentProfile>,
    context: Option<&SegmentContext>,
) -> Result<(LLMJudgment, i64), String> {
    let context_prompt = build_segment_user_prompt(text, context, doc_profile);
    let user_prompt = format!(
        "请分析以下文本是否由AI生成，并以JSON格式返回结果：\n\n[chunk_id={} start={} end={}]\n{}",
        chunk_id, start, end, context_prompt
    );

    let result = client
        // Use call_deepseek_json since prompt contains 'json'
        .call_deepseek_json(model, api_key, DETECTION_SYSTEM_PROMPT, &user_prompt, LLM_MAX_TOKENS)
        .await;

    match result {
        Ok(chat_result) => {
            let judgment = parse_single_judgment(&chat_result.content)?;
            Ok((judgment, chat_result.latency_ms))
        }
        Err(e) => {
            let msg = e.to_string();
            Err(format!("LLM call failed: {}", enrich_llm_call_error(&msg)))
        }
    }
}

async fn call_deepseek_for_sentence_with_retry(
    client: &ProviderClient,
    semaphore: &Semaphore,
    text: &str,
    api_key: &str,
    model: &str,
    chunk_id: i32,
    start: i32,
    end: i32,
    doc_profile: Option<&DocumentProfile>,
    context: Option<&SegmentContext>,
) -> Result<(LLMJudgment, usize, i64), String> {
    let mut last_err: Option<String> = None;

    for attempt in 1..=DEEPSEEK_SENTENCE_MAX_ATTEMPTS {
        let timeout_duration = std::time::Duration::from_secs(DEEPSEEK_SENTENCE_TIMEOUT_SECS);

        // Limit concurrent in-flight DeepSeek requests. Permit is held only for the request,
        // not for the entire retry window (so backoff doesn't waste concurrency slots).
        let res = {
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|_| "semaphore closed".to_string())?;
            let fut = call_deepseek_for_sentence(
                client,
                text,
                api_key,
                model,
                chunk_id,
                start,
                end,
                doc_profile,
                context,
            );
            tokio::time::timeout(timeout_duration, fut).await
        };

        match res {
            Ok(Ok((judgment, latency_ms))) => {
                info!(
                    "[LLM_ANALYZER] DeepSeek ok model={} chunk_id={} attempt={} latency_ms={}",
                    model, chunk_id, attempt, latency_ms
                );
                return Ok((judgment, attempt, latency_ms));
            }
            Ok(Err(e)) => {
                warn!(
                    "[LLM_ANALYZER] DeepSeek error model={} chunk_id={} attempt={} : {}",
                    model, chunk_id, attempt, e
                );
                last_err = Some(e);
            }
            Err(_) => {
                warn!(
                    "[LLM_ANALYZER] DeepSeek timeout model={} chunk_id={} attempt={} ({}s)",
                    model, chunk_id, attempt, DEEPSEEK_SENTENCE_TIMEOUT_SECS
                );
                last_err = Some("timeout".to_string());
            }
        };

        if attempt < DEEPSEEK_SENTENCE_MAX_ATTEMPTS {
            // Simple backoff to reduce rate-limit and transient failures.
            let backoff_ms = 400u64 * attempt as u64;
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        }
    }

    Err(last_err.unwrap_or_else(|| "unknown error".to_string()))
}

/// Call LLM to analyze multiple paragraphs in batch (GLM)
async fn call_llm_batch_paragraphs(
    client: &ProviderClient,
    paragraphs: &[(i32, String)], // (chunk_id, text)
    api_key: &str,
) -> Result<BatchLLMJudgment, String> {
    // Build user prompt with all paragraphs
    let mut user_prompt = String::from("请分析以下段落是否由AI生成：\n\n");
    for (chunk_id, text) in paragraphs {
        user_prompt.push_str(&format!("[段落 {}]\n{}\n\n", chunk_id, text));
    }

    let result = client
        .call_glm(
            "glm-4-flash",
            api_key,
            BATCH_DETECTION_SYSTEM_PROMPT,
            &user_prompt,
            LLM_MAX_TOKENS, // More tokens for batch response
            false,
        )
        .await;

    match result {
        Ok(chat_result) => parse_batch_judgment(&chat_result.content),
        Err(e) => Err(format!("GLM batch call failed: {}", e)),
    }
}

/// Parse single judgment JSON response
fn parse_single_judgment(content: &str) -> Result<LLMJudgment, String> {
    let content = content.trim();
    let json_str = extract_json(content)?;
    serde_json::from_str::<LLMJudgment>(&json_str)
        .map_err(|e| format!("JSON parse error: {}", e))
}

/// Parse batch judgment JSON response
fn parse_batch_judgment(content: &str) -> Result<BatchLLMJudgment, String> {
    let content = content.trim();
    let json_str = extract_json(content)?;
    serde_json::from_str::<BatchLLMJudgment>(&json_str)
        .map_err(|e| format!("JSON parse error: {}", e))
}

/// Extract JSON from response content
fn extract_json(content: &str) -> Result<String, String> {
    if content.starts_with('{') {
        Ok(content.to_string())
    } else if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            Ok(content[start..=end].to_string())
        } else {
            Err("Invalid JSON response".to_string())
        }
    } else {
        Err("No JSON in response".to_string())
    }
}

// ============================================================================
// Soft threshold functions for continuous scoring (same as segment_builder)
// ============================================================================

#[inline]
fn sigmoid(x: f64, center: f64, k: f64) -> f64 {
    1.0 / (1.0 + ((x - center) / k).exp())
}

#[inline]
fn sigmoid_inv(x: f64, center: f64, k: f64) -> f64 {
    1.0 - sigmoid(x, center, k)
}

#[inline]
fn from_logit(logit: f64) -> f64 {
    1.0 / (1.0 + (-logit).exp())
}

fn is_cjk_language(language: &str) -> bool {
    let lang = language.trim().to_ascii_lowercase();
    lang.starts_with("zh") || lang.starts_with("ja") || lang.starts_with("ko")
}

/// Calculate local stylometry-based probability for short sentences
/// Uses continuous soft-threshold algorithm (no perplexity for short text)
fn calculate_local_probability(
    text: &str,
    language: &str,
    doc_profile: Option<&DocumentProfile>,
) -> f64 {
    let metrics = compute_stylometry(text);
    let is_cjk = is_cjk_language(language);

    let ttr = metrics.ttr;
    let rep = metrics.repeat_ratio.unwrap_or(0.0);
    let ngram = metrics.ngram_repeat_rate.unwrap_or(0.0);
    let avg_len = metrics.avg_sentence_len;
    let academic_strength = doc_profile
        .filter(|p| is_academic_profile(p))
        .map(|p| match profile_validity(p) {
            ProfileValidity::Valid => 1.0,
            ProfileValidity::Partial => 0.6,
            ProfileValidity::Invalid => 0.0,
        })
        .unwrap_or(0.0);
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
    let ttr_low_weight = (1.0 - 0.25 * academic_strength) * if is_cjk { 0.85 } else { 1.0 };
    let rep_weight = (1.0 - 0.30 * academic_strength) * if is_cjk { 0.75 } else { 1.0 };
    let ngram_weight = (1.0 - 0.30 * academic_strength) * if is_cjk { 0.75 } else { 1.0 };
    let len_weight = if is_cjk { 0.85 } else { 1.0 };

    // Start in logit space
    let mut logit: f64 = 0.0;

    // TTR contribution (soft threshold)
    let ttr_low_contrib = sigmoid(ttr, ttr_low_center, ttr_low_k) * 1.0 * ttr_low_weight;
    let ttr_high_contrib = sigmoid_inv(ttr, ttr_high_center, ttr_high_k) * (-0.7);
    logit += ttr_low_contrib + ttr_high_contrib;

    // Repeat ratio contribution
    let rep_contrib = sigmoid_inv(rep, rep_center, rep_k) * 0.8 * rep_weight;
    logit += rep_contrib;

    // N-gram repeat contribution
    let ngram_contrib = sigmoid_inv(ngram, ngram_center, ngram_k) * 0.9 * ngram_weight;
    logit += ngram_contrib;

    // Sentence length contribution (U-shaped)
    let len_short_penalty = sigmoid(avg_len, len_short_center, len_short_k) * 0.25 * len_weight;
    let len_long_penalty = sigmoid_inv(avg_len, len_long_center, len_long_k) * 0.3 * len_weight;
    logit += len_short_penalty + len_long_penalty;

    from_logit(logit).clamp(0.02, 0.98)
}

/// Build document-level profile (discipline + summary) for context-aware detection.
pub async fn build_document_profile(
    text: &str,
    blocks: &[TextBlock],
    provider: Option<&str>,
) -> Option<DocumentProfile> {
    if text.chars().count() < 200 {
        return None;
    }

    let (provider_name, model, api_key) = resolve_provider_spec(provider)?;
    let client = build_provider_client_with_proxy();
    let custom_url = resolve_custom_url(&provider_name);

    match call_llm_document_profile_with_url(
        &client,
        text,
        blocks,
        &provider_name,
        &model,
        &api_key,
        custom_url.as_deref(),
    )
    .await
    {
        Ok(profile) => Some(profile),
        Err(e) => {
            warn!("[LLM_ANALYZER] Document profile failed: {}", e);
            None
        }
    }
}

/// Build segments with LLM analysis - BATCH mode for paragraphs via GLM
pub async fn build_paragraphs_batch_with_glm(
    text: &str,
    language: &str,
    blocks: &[TextBlock],
    use_perplexity: bool,
    use_stylometry: bool,
) -> Vec<SegmentResponse> {
    let started = Instant::now();
    info!(
        "[LLM_ANALYZER] Starting batch GLM analysis for {} blocks",
        blocks.len()
    );
    
    // Get GLM API key
    let api_key = match get_api_key("glm") {
        Some(k) => {
            info!("[LLM_ANALYZER] GLM API key found, length: {}", k.len());
            k
        }
        None => {
            warn!("[LLM_ANALYZER] GLM API key not configured, using local detection");
            return build_segments(text, language, blocks, use_perplexity, use_stylometry);
        }
    };

    let client = ProviderClient::new();
    let blocks_to_process: Vec<_> = blocks.iter().filter(|b| b.need_detect).collect();
    
    // Build segments first with local scores
    let mut segments: Vec<SegmentResponse> = blocks_to_process
        .iter()
        .enumerate()
        .map(|(idx, block)| {
            let block_text = &text[block.start as usize..block.end as usize];
            make_segment(
                idx as i32,
                language,
                block.start,
                block.end,
                block_text,
                use_perplexity,
                use_stylometry,
            )
        })
        .collect();
    
    if segments.is_empty() {
        return segments;
    }

    // Prepare paragraphs for batch call
    let paragraphs: Vec<(i32, String)> = blocks_to_process
        .iter()
        .enumerate()
        .map(|(idx, block)| {
            let block_text = text[block.start as usize..block.end as usize].to_string();
            (idx as i32, block_text)
        })
        .collect();
    let paragraph_lengths: Vec<usize> = paragraphs
        .iter()
        .map(|(_, block_text)| block_text.chars().count())
        .collect();

    // Call GLM batch API with timeout
    let timeout_duration = std::time::Duration::from_secs(120);
    let batch_future = call_llm_batch_paragraphs(&client, &paragraphs, &api_key);

    match tokio::time::timeout(timeout_duration, batch_future).await {
        Ok(Ok(batch_result)) => {
            info!(
                "[LLM_ANALYZER] GLM batch returned {} segments, elapsed_ms={}",
                batch_result.segments.len(),
                started.elapsed().as_millis()
            );
            // Apply batch results to segments
            for judgment in batch_result.segments {
                if let Some(seg) = segments.iter_mut().find(|s| s.chunk_id == judgment.chunk_id) {
                    let text_len = paragraph_lengths
                        .get(judgment.chunk_id as usize)
                        .copied()
                        .unwrap_or(0);
                    apply_llm_judgment_to_segment(
                        seg,
                        judgment.probability,
                        judgment.confidence,
                        judgment.uncertainty,
                        judgment.signals.clone(),
                        "glm:glm-4-flash".to_string(),
                        text_len,
                        None,
                    );
                    seg.explanations.push("batch_glm_analysis".to_string());
                }
            }
        }
        Ok(Err(e)) => {
            warn!("[LLM_ANALYZER] GLM batch analysis failed: {}", e);
        }
        Err(_) => {
            warn!("[LLM_ANALYZER] GLM batch analysis timeout (120s)");
        }
    }

    segments
}

/// Build segments for sentences with three-tier filtering via DeepSeek
/// - < 10 chars: Skip entirely
/// - 10-50 chars: Local stylometry scoring only
/// - >= 50 chars: Send to DeepSeek LLM (chat), and use reasoner for very long segments
pub async fn build_sentences_filtered_with_deepseek(
    text: &str,
    language: &str,
    blocks: &[TextBlock],
    use_perplexity: bool,
    use_stylometry: bool,
) -> Vec<SegmentResponse> {
    let started = Instant::now();
    info!(
        "[LLM_ANALYZER] Starting filtered DeepSeek analysis for {} blocks",
        blocks.len()
    );
    
    // Get DeepSeek API key
    let deepseek_key = get_api_key("deepseek");
    if deepseek_key.is_some() {
        info!("[LLM_ANALYZER] DeepSeek API key found");
    } else {
        warn!("[LLM_ANALYZER] DeepSeek API key NOT found, will use local scoring");
    }
    
    let client = Arc::new(ProviderClient::new());
    let blocks_to_process: Vec<_> = blocks.iter().filter(|b| b.need_detect).collect();
    let mut segments: Vec<SegmentResponse> = Vec::new();
    let mut chunk_id: i32 = 0;

    let semaphore = Arc::new(Semaphore::new(DEEPSEEK_SENTENCE_MAX_CONCURRENCY));
    let mut join_set: JoinSet<SegmentResponse> = JoinSet::new();
    let mut llm_tasks: usize = 0;

    for block in blocks_to_process.iter() {
        let block_text = &text[block.start as usize..block.end as usize];
        let char_count = block_text.chars().count();

        // Tier 1: Skip very short sentences (< 10 chars)
        if char_count < SENTENCE_MIN_LENGTH {
            continue;
        }

        let mut segment = make_segment(
            chunk_id,
            language,
            block.start,
            block.end,
            block_text,
            use_perplexity,
            use_stylometry,
        );

        // Tier 2: Local scoring for medium sentences (10-50 chars)
        if char_count < SENTENCE_LLM_THRESHOLD {
            segment.raw_probability = calculate_local_probability(block_text, language, None);
            segment.confidence = 0.5; // Lower confidence for local-only
            segment.uncertainty = (1.0 - segment.confidence).clamp(0.05, 0.9);
            segment.explanations.push("local_stylometry_only".to_string());
            segments.push(segment);
        } else {
            // Tier 3: Send to DeepSeek for long sentences (>= 50 chars)
            if let Some(ref api_key) = deepseek_key {
                // Concurrency-limited per-sentence DeepSeek calls.
                let model = if char_count >= SENTENCE_REASONER_THRESHOLD {
                    "deepseek-reasoner"
                } else {
                    "deepseek-chat"
                };

                let client = client.clone();
                let semaphore = semaphore.clone();
                let api_key = api_key.clone();
                let block_text = block_text.to_string();
                let start = block.start;
                let end = block.end;
                let current_chunk_id = chunk_id;
                let model = model;
                let language = language.to_string();
                let context = build_segment_context(block, blocks);
                llm_tasks += 1;

                join_set.spawn(async move {
                    // Default: local fallback if DeepSeek fails after retries.
                    let mut seg = segment;
                    let text_len = block_text.chars().count();

                    match call_deepseek_for_sentence_with_retry(
                        &client,
                        semaphore.as_ref(),
                        &block_text,
                        &api_key,
                        model,
                        current_chunk_id,
                        start,
                        end,
                        None,
                        Some(&context),
                    )
                    .await
                    {
                        Ok((judgment, attempt, _latency_ms)) => {
                            apply_llm_judgment_to_segment(
                                &mut seg,
                                judgment.probability,
                                judgment.confidence,
                                judgment.uncertainty,
                                judgment.signals.clone(),
                                format!("deepseek:{}", model),
                                text_len,
                                None,
                            );
                            if attempt > 1 {
                                seg.explanations.push(format!("deepseek_retry_success attempt={}", attempt));
                            } else {
                                seg.explanations.push(format!("deepseek_llm_analysis model={}", model));
                            }
                            if let Some(ref reason) = judgment.reasoning {
                                seg.explanations.push(reason.clone());
                            }
                        }
                        Err(e) => {
                            warn!(
                                "DeepSeek analysis failed for sentence {} ({}..{}) model={} : {}",
                                current_chunk_id, start, end, model, e
                            );
                            seg.raw_probability = calculate_local_probability(&block_text, &language, None);
                            seg.confidence = 0.4;
                            seg.uncertainty = (1.0 - seg.confidence).clamp(0.05, 0.9);
                            seg.explanations.push("deepseek_retry_exhausted_local_fallback".to_string());
                        }
                    }

                    seg
                });
            } else {
                // No DeepSeek key, use local scoring
                segment.raw_probability = calculate_local_probability(block_text, language, None);
                segment.confidence = 0.5;
                segment.uncertainty = (1.0 - segment.confidence).clamp(0.05, 0.9);
                segment.explanations.push("no_deepseek_key_local_only".to_string());
                segments.push(segment);
            }
        }
        chunk_id += 1;
    }

    let mut done_llm_tasks: usize = 0;
    while let Some(res) = join_set.join_next().await {
        done_llm_tasks += 1;
        if llm_tasks > 0 && (done_llm_tasks == llm_tasks || done_llm_tasks % 5 == 0) {
            info!(
                "[LLM_ANALYZER] DeepSeek progress: {}/{} (elapsed_ms={})",
                done_llm_tasks,
                llm_tasks,
                started.elapsed().as_millis()
            );
        }
        match res {
            Ok(seg) => segments.push(seg),
            Err(e) => warn!("[LLM_ANALYZER] DeepSeek sentence task failed: {}", e),
        }
    }

    // Ensure stable order for downstream comparison/UX.
    segments.sort_by_key(|s| s.chunk_id);
    info!(
        "[LLM_ANALYZER] DeepSeek sentence analysis done: segments={}, llm_tasks={}, elapsed_ms={}",
        segments.len(),
        llm_tasks,
        started.elapsed().as_millis()
    );
    segments
}

/// Original build_segments_with_llm - kept for backward compatibility
pub async fn build_segments_with_llm(
    text: &str,
    language: &str,
    blocks: &[TextBlock],
    use_perplexity: bool,
    use_stylometry: bool,
    provider: Option<&str>,
    doc_profile: Option<&DocumentProfile>,
) -> Vec<SegmentResponse> {
    build_segments_with_llm_progress(
        text,
        language,
        blocks,
        use_perplexity,
        use_stylometry,
        provider,
        None,
        doc_profile,
        |_, _| {},
    )
    .await
}

/// Build segments with LLM analysis and progress callback
pub async fn build_segments_with_llm_progress<F>(
    text: &str,
    language: &str,
    blocks: &[TextBlock],
    use_perplexity: bool,
    use_stylometry: bool,
    provider: Option<&str>,
    all_blocks: Option<&[TextBlock]>,
    doc_profile: Option<&DocumentProfile>,
    on_progress: F,
) -> Vec<SegmentResponse>
where
    F: Fn(usize, usize),
{
    let provider = provider.map(|p| p.trim()).filter(|p| !p.is_empty());
    let (provider_name, model, api_key) = match resolve_provider_spec(provider) {
        Some(info) => info,
        None => {
            warn!("No API key configured, using local detection");
            return build_segments_with_profile(
                text,
                language,
                blocks,
                use_perplexity,
                use_stylometry,
                doc_profile,
            );
        }
    };
    let client = build_provider_client_with_proxy();
    let custom_url = resolve_custom_url(&provider_name);

    let client = Arc::new(client);
    let semaphore = llm_segment_semaphore().clone();

    let mut segments: Vec<SegmentResponse> = Vec::new();
    let blocks_to_process: Vec<_> = blocks.iter().filter(|b| b.need_detect).collect();
    let mut join_set: JoinSet<SegmentResponse> = JoinSet::new();
    let context_blocks = all_blocks.unwrap_or(blocks);

    let started = Instant::now();
    info!(
        "[LLM_ANALYZER] segments_with_llm start provider={} model={} blocks={} max_concurrency={}",
        provider_name,
        model,
        blocks_to_process.len(),
        LLM_SEGMENT_MAX_CONCURRENCY
    );

    for (idx, block) in blocks_to_process.iter().enumerate() {
        let client = Arc::clone(&client);
        let semaphore = Arc::clone(&semaphore);
        let provider_name = provider_name.clone();
        let model = model.clone();
        let api_key = api_key.clone();
        let language = language.to_string();
        let block_text = text[block.start as usize..block.end as usize].to_string();
        let start = block.start;
        let end = block.end;
        let custom_url = custom_url.clone();
        let timeout_duration = std::time::Duration::from_secs(120);
        let context = build_segment_context(block, context_blocks);
        let doc_profile = doc_profile.cloned();

        join_set.spawn(async move {
            let mut segment = make_segment_with_profile(
                idx as i32,
                &language,
                start,
                end,
                &block_text,
                use_perplexity,
                use_stylometry,
                doc_profile.as_ref(),
            );
            let text_len = block_text.chars().count();

            let permit = semaphore.acquire().await;
            if permit.is_err() {
                segment.raw_probability = calculate_local_probability(&block_text, &language, doc_profile.as_ref());
                segment.confidence = 0.5;
                segment.uncertainty = (1.0 - segment.confidence).clamp(0.05, 0.9);
                segment.explanations.push("semaphore_closed_local_only".to_string());
                return segment;
            }
            let _permit = permit.unwrap();

            let llm_future = call_llm_for_segment_with_url(
                &client,
                &block_text,
                &provider_name,
                &model,
                &api_key,
                custom_url.as_deref(),
                doc_profile.as_ref(),
                Some(&context),
            );

            match tokio::time::timeout(timeout_duration, llm_future).await {
                Ok(Ok(judgment)) => {
                    apply_llm_judgment_to_segment(
                        &mut segment,
                        judgment.probability,
                        judgment.confidence,
                        judgment.uncertainty,
                        judgment.signals,
                        format!("{}:{}", provider_name, model),
                        text_len,
                        doc_profile.as_ref(),
                    );
                    if let Some(reason) = judgment.reasoning {
                        segment.explanations.push(reason);
                    }
                }
                Ok(Err(e)) => {
                    warn!("[LLM_ANALYZER] LLM analysis failed for segment {}: {}", idx, e);
                    segment.uncertainty = segment.uncertainty.max(0.5);
                    segment.explanations.push("llm_failed_local_only".to_string());
                }
                Err(_) => {
                    warn!("[LLM_ANALYZER] LLM analysis timeout for segment {}", idx);
                    segment.uncertainty = segment.uncertainty.max(0.5);
                    segment.explanations.push("llm_timeout_local_only".to_string());
                }
            }

            segment
        });
    }

    let total = blocks_to_process.len();
    let mut done = 0usize;

    while let Some(res) = join_set.join_next().await {
        done += 1;
        on_progress(done, total);
        match res {
            Ok(seg) => segments.push(seg),
            Err(e) => warn!("[LLM_ANALYZER] segment task failed: {}", e),
        }
    }

    segments.sort_by_key(|s| s.chunk_id);
    info!(
        "[LLM_ANALYZER] segments_with_llm done segments={} elapsed_ms={}",
        segments.len(),
        started.elapsed().as_millis()
    );

    segments
}
