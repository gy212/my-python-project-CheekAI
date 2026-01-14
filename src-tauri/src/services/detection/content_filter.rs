// Content Filter Module
// Hybrid filtering to identify and skip non-body content (titles, TOC, references, etc.)

use tracing::{info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::services::providers::{get_api_key, parse_provider, ProviderClient, OPENAI_DEFAULT_MODEL};
use crate::services::ConfigStore;
use crate::services::text_processor::TextBlock;

/// Paragraph classification category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphCategory {
    Body,      // Main content - needs detection
    Title,     // Titles, headings
    Toc,       // Table of contents
    Reference, // References, bibliography
    Auxiliary, // Acknowledgments, appendix, author info
    Noise,     // Captions, numeric data, etc.
}

/// Classification result for a single paragraph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphClassification {
    pub index: i32,
    pub category: ParagraphCategory,
    pub confidence: f64,
    pub reason: String,
}

/// Filter summary for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSummary {
    pub total_paragraphs: i32,
    pub body_count: i32,
    pub filtered_count: i32,
    pub filtered_by_rule: i32,
    pub filtered_by_llm: i32,
    pub classifications: Vec<ParagraphClassification>,
}

const FILTER_MAX_TOKENS: i32 = 2048;

const FILTER_SYSTEM_PROMPT: &str = r#"你是一个文档结构分析专家。请判断以下段落属于哪种类型：
- body: 正文内容（需要检测）
- title: 标题、章节名
- toc: 目录
- reference: 参考文献
- auxiliary: 致谢、附录、作者信息等辅助内容

请以JSON格式返回结果，包含一个paragraphs数组，每个元素包含：
- index: 段落编号
- category: 类型（body/title/toc/reference/auxiliary）
- confidence: 置信度（0-1）

示例：{"paragraphs": [{"index": 0, "category": "body", "confidence": 0.95}]}
只返回JSON，不要有其他文字。"#;

fn resolve_custom_url(provider_name: &str) -> Option<String> {
    let store = ConfigStore::default_config_dir().map(ConfigStore::new)?;
    store
        .get_provider_url(provider_name)
        .ok()
        .flatten()
        .or_else(|| {
            if provider_name == "anthropic" {
                store.get_provider_url("claude").ok().flatten()
            } else if provider_name == "claude" {
                store.get_provider_url("anthropic").ok().flatten()
            } else {
                None
            }
        })
}

fn extract_json(content: &str) -> String {
    let trimmed = content.trim();

    // Try to find JSON object
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }

    trimmed.to_string()
}

fn select_provider_for_filter(provider: Option<&str>) -> Option<(String, String, String)> {
    let provider = provider.map(|p| p.trim()).filter(|p| !p.is_empty());

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

/// Check if text has sentence-ending punctuation
fn has_sentence_end_punctuation(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    let last_char = trimmed.chars().last().unwrap();
    matches!(last_char, '。' | '！' | '？' | '.' | '!' | '?')
}

/// Check if text matches title pattern
fn is_title_pattern(s: &str) -> bool {
    let trimmed = s.trim();
    let char_count = trimmed.chars().count();

    // Too long for a title
    if char_count > 60 {
        return false;
    }

    // Has sentence-ending punctuation - likely not a title
    if has_sentence_end_punctuation(trimmed) {
        return false;
    }

    // Numbered heading patterns
    let numbered_re = Regex::new(
        r"^(\d+\.?\d*\.?\d*\s|第[一二三四五六七八九十百千]+[章节部分条款]|Chapter\s+\d+|Section\s+\d+|Part\s+\d+)",
    )
    .unwrap();
    if numbered_re.is_match(trimmed) {
        return true;
    }

    // Short text without punctuation (likely a title)
    if char_count <= 30 && !has_sentence_end_punctuation(trimmed) {
        // Check if it's mostly letters/Chinese characters (not numbers)
        let alpha_count = trimmed
            .chars()
            .filter(|c| c.is_alphabetic() || *c > '\u{4E00}')
            .count();
        if alpha_count as f64 / char_count.max(1) as f64 > 0.7 {
            return true;
        }
    }

    false
}

/// Check if text matches TOC (table of contents) pattern
fn is_toc_pattern(s: &str) -> bool {
    let trimmed = s.trim();
    let lower = trimmed.to_lowercase();

    // Standalone TOC headers
    if lower == "目录" || lower == "contents" || lower == "table of contents" {
        return true;
    }

    // Contains page numbers with dots/dashes: "Introduction.....1" or "Introduction --- 1"
    let toc_re = Regex::new(r"[\.·\-]{3,}\s*\d+\s*$").unwrap();
    if toc_re.is_match(trimmed) {
        return true;
    }

    // Pattern like "1.1 Introduction 5" (chapter + title + page number)
    let toc_line_re = Regex::new(r"^\d+\.?\d*\.?\d*\s+.+\s+\d+$").unwrap();
    if toc_line_re.is_match(trimmed) && trimmed.chars().count() < 80 {
        return true;
    }

    false
}

/// Check if text matches reference/bibliography pattern
fn is_reference_pattern(s: &str) -> bool {
    let trimmed = s.trim();
    let lower = trimmed.to_lowercase();

    // Standalone reference headers
    if lower == "参考文献"
        || lower == "references"
        || lower == "bibliography"
        || lower == "works cited"
    {
        return true;
    }

    // Starts with [1], [2], etc.
    let bracket_num_re = Regex::new(r"^\[\d+\]").unwrap();
    if bracket_num_re.is_match(trimmed) {
        return true;
    }

    // Author-year format: "Smith, J. (2020)" or "张三. (2020)"
    let author_year_re =
        Regex::new(r"^[A-Z\u{4E00}-\u{9FFF}][a-z\u{4E00}-\u{9FFF}]*[,，]?\s*.+[\(\[]?\d{4}[\)\]]?")
            .unwrap();
    if author_year_re.is_match(trimmed) && trimmed.contains("http") == false {
        // Check if it looks like a citation (has year and possibly DOI/URL)
        let has_year = Regex::new(r"\b(19|20)\d{2}\b").unwrap().is_match(trimmed);
        let has_journal_markers = trimmed.contains("Vol.")
            || trimmed.contains("pp.")
            || trimmed.contains("doi:")
            || trimmed.contains("ISBN");
        if has_year && (has_journal_markers || trimmed.chars().count() < 200) {
            return true;
        }
    }

    false
}

/// Check if text is noise (captions, numeric data, etc.)
/// Adapted from is_noise_paragraph in detect.rs
fn is_noise_pattern(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return true;
    }

    let total = trimmed.chars().count() as f64;

    // Figure/table captions
    let caption_re = Regex::new(r"^(图|表|Figure|Fig\.|Table)\s*\d+").unwrap();
    if caption_re.is_match(trimmed) {
        return true;
    }

    // High digit ratio (> 60%)
    let digit_count = trimmed.chars().filter(|c| c.is_ascii_digit()).count() as f64;
    let digit_ratio = digit_count / total.max(1.0);
    if digit_ratio > 0.6 {
        return true;
    }

    // Short text with high digit ratio and no sentence ending
    if total < 25.0 && !has_sentence_end_punctuation(trimmed) && digit_ratio > 0.3 {
        return true;
    }

    // Symbol soup (very low letter ratio)
    let letter_count = trimmed
        .chars()
        .filter(|c| c.is_alphabetic() || *c > '\u{4E00}')
        .count() as f64;
    let letter_ratio = letter_count / total.max(1.0);
    if letter_ratio < 0.1 && total < 50.0 {
        return true;
    }

    false
}

/// Classify a paragraph using rule-based patterns
/// Returns Some(classification) if confidently classified, None if uncertain
pub fn classify_by_rules(text: &str, index: i32) -> Option<ParagraphClassification> {
    let trimmed = text.trim();

    // Noise patterns (highest priority)
    if is_noise_pattern(trimmed) {
        return Some(ParagraphClassification {
            index,
            category: ParagraphCategory::Noise,
            confidence: 0.9,
            reason: "noise_pattern".to_string(),
        });
    }

    // TOC patterns
    if is_toc_pattern(trimmed) {
        return Some(ParagraphClassification {
            index,
            category: ParagraphCategory::Toc,
            confidence: 0.95,
            reason: "toc_pattern".to_string(),
        });
    }

    // Reference patterns
    if is_reference_pattern(trimmed) {
        return Some(ParagraphClassification {
            index,
            category: ParagraphCategory::Reference,
            confidence: 0.9,
            reason: "reference_pattern".to_string(),
        });
    }

    // Title patterns
    if is_title_pattern(trimmed) {
        return Some(ParagraphClassification {
            index,
            category: ParagraphCategory::Title,
            confidence: 0.85,
            reason: "title_pattern".to_string(),
        });
    }

    // Long enough text with sentence ending - likely body
    let char_count = trimmed.chars().count();
    if char_count > 100 && has_sentence_end_punctuation(trimmed) {
        return Some(ParagraphClassification {
            index,
            category: ParagraphCategory::Body,
            confidence: 0.9,
            reason: "body_pattern".to_string(),
        });
    }

    // Uncertain - needs LLM classification
    None
}

/// LLM response structure for batch classification
#[derive(Debug, Deserialize)]
struct LlmClassificationResponse {
    paragraphs: Vec<LlmParagraphResult>,
}

#[derive(Debug, Deserialize)]
struct LlmParagraphResult {
    index: i32,
    category: String,
    confidence: f64,
}

/// Classify uncertain paragraphs using LLM (batch call)
pub async fn classify_by_llm(
    client: &ProviderClient,
    paragraphs: &[(i32, String)],
    provider: Option<&str>,
) -> Result<Vec<ParagraphClassification>, String> {
    if paragraphs.is_empty() {
        return Ok(Vec::new());
    }

    let (provider_name, model, api_key) = select_provider_for_filter(provider)
        .ok_or_else(|| "No API key available for LLM filtering".to_string())?;
    let custom_url = resolve_custom_url(&provider_name);

    // Build user prompt with truncated previews
    let mut user_prompt = String::from("请分析以下段落的类型：\n\n");
    for (idx, text) in paragraphs {
        let preview = if text.chars().count() > 200 {
            format!("{}...", text.chars().take(200).collect::<String>())
        } else {
            text.clone()
        };
        user_prompt.push_str(&format!("[段落 {}]\n{}\n\n", idx, preview));
    }

    let result = match provider_name.as_str() {
        "glm" => client
            .call_glm_with_url(
                custom_url.as_deref(),
                &model,
                &api_key,
                FILTER_SYSTEM_PROMPT,
                &user_prompt,
                FILTER_MAX_TOKENS,
                false,
            )
            .await
            .map(|r| r.content),
        "deepseek" => client
            .call_deepseek_json_with_url(
                custom_url.as_deref(),
                &model,
                &api_key,
                FILTER_SYSTEM_PROMPT,
                &user_prompt,
                FILTER_MAX_TOKENS,
            )
            .await
            .map(|r| r.content),
        "openai" => {
            let combined = format!("{}\n\n{}", FILTER_SYSTEM_PROMPT, user_prompt);
            client
                .call_openai_responses_with_url(custom_url.as_deref(), &model, &api_key, &combined)
                .await
                .map(|r| r.content)
        }
        "gemini" => client
            .call_gemini_with_url(
                custom_url.as_deref(),
                &model,
                &api_key,
                FILTER_SYSTEM_PROMPT,
                &user_prompt,
                FILTER_MAX_TOKENS,
            )
            .await
            .map(|r| r.content),
        "anthropic" | "claude" => client
            .call_anthropic_with_url(
                custom_url.as_deref(),
                &model,
                &api_key,
                FILTER_SYSTEM_PROMPT,
                &user_prompt,
                FILTER_MAX_TOKENS,
            )
            .await
            .map(|r| r.content),
        _ => {
            let combined = format!("{}\n\n{}", FILTER_SYSTEM_PROMPT, user_prompt);
            client
                .call_openai_responses_with_url(custom_url.as_deref(), &model, &api_key, &combined)
                .await
                .map(|r| r.content)
        }
    };

    match result {
        Ok(content) => parse_classification_response(&content, paragraphs),
        Err(e) => Err(format!("LLM classification failed: {}", e)),
    }
}

/// Parse LLM response into classification results
fn parse_classification_response(
    content: &str,
    paragraphs: &[(i32, String)],
) -> Result<Vec<ParagraphClassification>, String> {
    // Try to extract JSON from response
    let json_str = extract_json(content);

    let parsed: LlmClassificationResponse = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse LLM response: {} - content: {}", e, content))?;

    let mut results = Vec::new();
    for item in parsed.paragraphs {
        let category = match item.category.as_str() {
            "body" => ParagraphCategory::Body,
            "title" => ParagraphCategory::Title,
            "toc" => ParagraphCategory::Toc,
            "reference" => ParagraphCategory::Reference,
            "auxiliary" => ParagraphCategory::Auxiliary,
            _ => ParagraphCategory::Body, // Default to body for unknown categories
        };

        results.push(ParagraphClassification {
            index: item.index,
            category,
            confidence: item.confidence,
            reason: "llm_classification".to_string(),
        });
    }

    // Fill in any missing paragraphs with default body classification
    for (idx, _) in paragraphs {
        if !results.iter().any(|r| r.index == *idx) {
            results.push(ParagraphClassification {
                index: *idx,
                category: ParagraphCategory::Body,
                confidence: 0.5,
                reason: "llm_missing_default".to_string(),
            });
        }
    }

    Ok(results)
}

/// Main hybrid filtering function
/// Returns (filtered_blocks, filter_summary)
pub async fn filter_paragraphs(
    blocks: &[TextBlock],
    provider: Option<&str>,
) -> (Vec<TextBlock>, FilterSummary) {
    let mut body_blocks = Vec::new();
    let mut classifications = Vec::new();
    let mut uncertain_paragraphs: Vec<(i32, String)> = Vec::new();
    let mut filtered_by_rule = 0;

    // Phase 1: Rule-based classification
    for block in blocks {
        if let Some(classification) = classify_by_rules(&block.text, block.index) {
            classifications.push(classification.clone());
            if classification.category == ParagraphCategory::Body {
                body_blocks.push(block.clone());
            } else {
                filtered_by_rule += 1;
            }
        } else {
            uncertain_paragraphs.push((block.index, block.text.clone()));
        }
    }

    info!(
        "[content_filter] Rule-based: {} body, {} filtered, {} uncertain",
        body_blocks.len(),
        filtered_by_rule,
        uncertain_paragraphs.len()
    );

    // Phase 2: LLM classification for uncertain paragraphs
    let mut filtered_by_llm = 0;
    if !uncertain_paragraphs.is_empty() {
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

        let client = match proxy_url.as_deref() {
            Some(p) => ProviderClient::with_proxy(p).unwrap_or_else(|_| ProviderClient::new()),
            None => ProviderClient::new(),
        };

        match classify_by_llm(&client, &uncertain_paragraphs, provider).await {
            Ok(llm_classifications) => {
                for classification in llm_classifications {
                    classifications.push(classification.clone());
                    if classification.category == ParagraphCategory::Body {
                        if let Some(block) = blocks.iter().find(|b| b.index == classification.index) {
                            body_blocks.push(block.clone());
                        }
                    } else {
                        filtered_by_llm += 1;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "[content_filter] LLM classification failed, keeping uncertain as body: {}",
                    e
                );
                // Fallback: keep uncertain paragraphs as body
                for (idx, _) in &uncertain_paragraphs {
                    if let Some(block) = blocks.iter().find(|b| b.index == *idx) {
                        body_blocks.push(block.clone());
                    }
                    classifications.push(ParagraphClassification {
                        index: *idx,
                        category: ParagraphCategory::Body,
                        confidence: 0.5,
                        reason: "llm_fallback".to_string(),
                    });
                }
            }
        }
    }

    // Sort body blocks by index to maintain order
    body_blocks.sort_by_key(|b| b.index);

    // Sort classifications by index
    classifications.sort_by_key(|c| c.index);

    let summary = FilterSummary {
        total_paragraphs: blocks.len() as i32,
        body_count: body_blocks.len() as i32,
        filtered_count: (blocks.len() - body_blocks.len()) as i32,
        filtered_by_rule,
        filtered_by_llm,
        classifications,
    };

    info!(
        "[content_filter] Final: {} -> {} paragraphs (rule: -{}, llm: -{})",
        summary.total_paragraphs, summary.body_count, filtered_by_rule, filtered_by_llm
    );

    (body_blocks, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_title_pattern() {
        assert!(is_title_pattern("第一章 绪论"));
        assert!(is_title_pattern("1.1 研究背景"));
        assert!(is_title_pattern("Chapter 1 Introduction"));
        assert!(is_title_pattern("摘要"));
        assert!(!is_title_pattern("这是一段正文内容，包含完整的句子。"));
    }

    #[test]
    fn test_is_toc_pattern() {
        assert!(is_toc_pattern("目录"));
        assert!(is_toc_pattern("Contents"));
        assert!(is_toc_pattern("1.1 Introduction.....5"));
        assert!(is_toc_pattern("第一章 绪论 --- 1"));
        assert!(!is_toc_pattern("这是正文内容。"));
    }

    #[test]
    fn test_is_reference_pattern() {
        assert!(is_reference_pattern("参考文献"));
        assert!(is_reference_pattern("[1] Smith, J. (2020). Title. Journal."));
        assert!(is_reference_pattern("[2] 张三. 论文标题[J]. 期刊, 2020."));
        assert!(!is_reference_pattern("这是正文内容。"));
    }

    #[test]
    fn test_classify_by_rules() {
        // Title
        let result = classify_by_rules("第一章 绪论", 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, ParagraphCategory::Title);

        // Body (long text with punctuation)
        let long_text = "这是一段很长的正文内容，包含了完整的句子和标点符号。这段文字足够长，可以被识别为正文内容。这是第三句话，用来确保文本长度超过阈值。";
        let result = classify_by_rules(long_text, 1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, ParagraphCategory::Body);

        // Uncertain (short text without clear pattern)
        let result = classify_by_rules("一些不确定的内容", 2);
        assert!(result.is_none());
    }
}
