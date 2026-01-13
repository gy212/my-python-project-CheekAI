// LLM Analyzer
// Handles LLM-based text analysis for AI detection
// Supports:
// - Batch paragraph analysis via GLM
// - Sentence-level analysis via DeepSeek with length filtering

use crate::models::{SegmentResponse, SignalLLMJudgment};
use crate::services::providers::{get_api_key, parse_provider, ProviderClient};
use crate::services::text_processor::{compute_stylometry, TextBlock};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{info, warn};

use super::segment_builder::{build_segments, make_segment};

/// Sentence length thresholds (Unicode scalar count, not UTF-8 byte length)
const SENTENCE_MIN_LENGTH: usize = 10;  // Skip sentences shorter than this
const SENTENCE_LLM_THRESHOLD: usize = 50;  // Send to LLM if >= this length
const SENTENCE_REASONER_THRESHOLD: usize = 300; // Use DeepSeek reasoner for very long segments
const DEEPSEEK_SENTENCE_MAX_CONCURRENCY: usize = 4;
const DEEPSEEK_SENTENCE_MAX_ATTEMPTS: usize = 3; // initial + retries
const DEEPSEEK_SENTENCE_TIMEOUT_SECS: u64 = 60;

/// System prompt for single segment AI detection
const DETECTION_SYSTEM_PROMPT: &str = r#"你是一个专业的AI文本检测专家。你需要判断给定的文本是否由AI生成。
请分析文本的以下特征：
1. 语言流畅度和自然程度
2. 是否存在AI生成文本的典型特征（如过度正式、缺乏个人风格、重复模式等）
3. 内容的逻辑性和连贯性

请以JSON格式返回结果，包含以下字段：
- probability: 0.000-1.000之间的三位小数，表示文本是AI生成的概率（例如0.423, 0.781, 0.156）
- confidence: 0.000-1.000之间的三位小数，表示你对判断的置信度
- reasoning: 简短的分析说明

重要：probability和confidence必须是精确到三位小数的数值，不要使用整数或一位小数。
只返回JSON，不要有其他文字。"#;

/// System prompt for batch paragraph AI detection (GLM)
const BATCH_DETECTION_SYSTEM_PROMPT: &str = r#"你是一个专业的AI文本检测专家。你需要判断给定的多个文本段落是否由AI生成。

请分析每个段落的以下特征：
1. 语言流畅度和自然程度
2. 是否存在AI生成文本的典型特征（如过度正式、缺乏个人风格、重复模式等）
3. 内容的逻辑性和连贯性

请以JSON格式返回结果，包含一个segments数组，每个元素包含：
- chunk_id: 段落编号（从0开始）
- probability: 0.000-1.000之间的三位小数，表示该段落是AI生成的概率
- confidence: 0.000-1.000之间的三位小数，表示你对该判断的置信度

重要：probability和confidence必须是精确到三位小数的数值，例如0.423、0.781、0.156，不要使用整数或一位小数如0.4、0.8。

示例格式：
{"segments": [{"chunk_id": 0, "probability": 0.723, "confidence": 0.856}, {"chunk_id": 1, "probability": 0.312, "confidence": 0.945}]}

只返回JSON，不要有其他文字。"#;

/// LLM judgment response for single segment
#[derive(Debug, Deserialize, Default)]
struct LLMJudgment {
    #[serde(default)]
    probability: f64,
    #[serde(default = "default_confidence")]
    confidence: f64,
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
}

fn default_confidence() -> f64 {
    0.6
}

/// Call LLM to analyze text segment (single)
async fn call_llm_for_segment(
    client: &ProviderClient,
    text: &str,
    provider_name: &str,
    model: &str,
    api_key: &str,
) -> Result<LLMJudgment, String> {
    let user_prompt = format!("请分析以下文本是否由AI生成，并以JSON格式返回结果：\n\n{}", text);

    let result = if provider_name == "deepseek" {
        // Use call_deepseek_json since prompt contains 'json'
        client
            .call_deepseek_json(model, api_key, DETECTION_SYSTEM_PROMPT, &user_prompt, 512)
            .await
    } else if provider_name == "gemini" {
        client
            .call_gemini(model, api_key, DETECTION_SYSTEM_PROMPT, &user_prompt, 512)
            .await
    } else {
        client
            .call_glm(
                model,
                api_key,
                DETECTION_SYSTEM_PROMPT,
                &user_prompt,
                512,
                false,
            )
            .await
    };

    match result {
        Ok(chat_result) => parse_single_judgment(&chat_result.content),
        Err(e) => Err(format!("LLM call failed: {}", e)),
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
) -> Result<(LLMJudgment, i64), String> {
    let user_prompt = format!(
        "请分析以下文本是否由AI生成，并以JSON格式返回结果：\n\n[chunk_id={} start={} end={}]\n{}",
        chunk_id, start, end, text
    );

    let result = client
        // Use call_deepseek_json since prompt contains 'json'
        .call_deepseek_json(model, api_key, DETECTION_SYSTEM_PROMPT, &user_prompt, 512)
        .await;

    match result {
        Ok(chat_result) => {
            let judgment = parse_single_judgment(&chat_result.content)?;
            Ok((judgment, chat_result.latency_ms))
        }
        Err(e) => Err(format!("LLM call failed: {}", e)),
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
            let fut = call_deepseek_for_sentence(client, text, api_key, model, chunk_id, start, end);
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
            2048, // More tokens for batch response
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

/// Calculate local stylometry-based probability for short sentences
/// Uses continuous soft-threshold algorithm (no perplexity for short text)
fn calculate_local_probability(text: &str) -> f64 {
    let metrics = compute_stylometry(text);

    let ttr = metrics.ttr;
    let rep = metrics.repeat_ratio.unwrap_or(0.0);
    let ngram = metrics.ngram_repeat_rate.unwrap_or(0.0);
    let avg_len = metrics.avg_sentence_len;

    // Start in logit space
    let mut logit: f64 = 0.0;

    // TTR contribution (soft threshold)
    let ttr_low_contrib = sigmoid(ttr, 0.58, 0.08) * 1.0;
    let ttr_high_contrib = sigmoid_inv(ttr, 0.78, 0.06) * (-0.7);
    logit += ttr_low_contrib + ttr_high_contrib;

    // Repeat ratio contribution
    let rep_contrib = sigmoid_inv(rep, 0.18, 0.06) * 0.8;
    logit += rep_contrib;

    // N-gram repeat contribution
    let ngram_contrib = sigmoid_inv(ngram, 0.10, 0.04) * 0.9;
    logit += ngram_contrib;

    // Sentence length contribution (U-shaped)
    let len_short_penalty = sigmoid(avg_len, 35.0, 10.0) * 0.25;
    let len_long_penalty = sigmoid_inv(avg_len, 120.0, 25.0) * 0.3;
    logit += len_short_penalty + len_long_penalty;

    from_logit(logit).clamp(0.02, 0.98)
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
                    seg.ai_probability = judgment.probability.clamp(0.0, 1.0);
                    seg.confidence = judgment.confidence.clamp(0.0, 1.0);
                    seg.signals.llm_judgment = SignalLLMJudgment {
                        prob: Some(judgment.probability),
                        models: vec!["glm:glm-4-flash".to_string()],
                    };
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
            segment.ai_probability = calculate_local_probability(block_text);
            segment.confidence = 0.5; // Lower confidence for local-only
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
                llm_tasks += 1;

                join_set.spawn(async move {
                    // Default: local fallback if DeepSeek fails after retries.
                    let mut seg = segment;

                    match call_deepseek_for_sentence_with_retry(
                        &client,
                        semaphore.as_ref(),
                        &block_text,
                        &api_key,
                        model,
                        current_chunk_id,
                        start,
                        end,
                    )
                    .await
                    {
                        Ok((judgment, attempt, _latency_ms)) => {
                            seg.ai_probability = judgment.probability.clamp(0.0, 1.0);
                            seg.confidence = judgment.confidence.clamp(0.0, 1.0);
                            seg.signals.llm_judgment = SignalLLMJudgment {
                                prob: Some(judgment.probability),
                                models: vec![format!("deepseek:{}", model)],
                            };
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
                            seg.ai_probability = calculate_local_probability(&block_text);
                            seg.confidence = 0.4;
                            seg.explanations.push("deepseek_retry_exhausted_local_fallback".to_string());
                        }
                    }

                    seg
                });
            } else {
                // No DeepSeek key, use local scoring
                segment.ai_probability = calculate_local_probability(block_text);
                segment.confidence = 0.5;
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
) -> Vec<SegmentResponse> {
    // Get provider info and API key
    let provider_info = if let Some(p) = provider {
        let spec = parse_provider(p);
        let model = if spec.model.trim().is_empty() {
            match spec.name.as_str() {
                "gemini" => "gemini-3-pro-preview".to_string(),
                "glm" => "glm-4-flash".to_string(),
                "deepseek" => "deepseek-chat".to_string(),
                _ => spec.model,
            }
        } else {
            spec.model
        };
        let key = get_api_key(&spec.name);
        if let Some(k) = key {
            Some((spec.name, model, k))
        } else {
            None
        }
    } else {
        // Try Gemini first, then GLM, then DeepSeek
        if let Some(key) = get_api_key("gemini") {
            Some(("gemini".to_string(), "gemini-3-pro-preview".to_string(), key))
        } else if let Some(key) = get_api_key("glm") {
            Some(("glm".to_string(), "glm-4-flash".to_string(), key))
        } else if let Some(key) = get_api_key("deepseek") {
            Some(("deepseek".to_string(), "deepseek-chat".to_string(), key))
        } else {
            None
        }
    };

    // If no API key available, use non-LLM detection
    let (provider_name, model, api_key) = match provider_info {
        Some(info) => info,
        None => {
            warn!("No API key configured, using local detection");
            return build_segments(text, language, blocks, use_perplexity, use_stylometry);
        }
    };

    let client = ProviderClient::new();
    let mut segments = Vec::new();
    let blocks_to_process: Vec<_> = blocks.iter().filter(|b| b.need_detect).collect();

    for (idx, block) in blocks_to_process.iter().enumerate() {
        let block_text = &text[block.start as usize..block.end as usize];

        // Create base segment
        let mut segment = make_segment(
            idx as i32,
            language,
            block.start,
            block.end,
            block_text,
            use_perplexity,
            use_stylometry,
        );

        // Use tokio timeout to prevent hanging (60 seconds per segment)
        let timeout_duration = std::time::Duration::from_secs(60);
        let llm_future = call_llm_for_segment(&client, block_text, &provider_name, &model, &api_key);

        match tokio::time::timeout(timeout_duration, llm_future).await {
            Ok(Ok(judgment)) => {
                segment.ai_probability = judgment.probability.clamp(0.0, 1.0);
                segment.confidence = judgment.confidence.clamp(0.0, 1.0);
                segment.signals.llm_judgment = SignalLLMJudgment {
                    prob: Some(judgment.probability),
                    models: vec![format!("{}:{}", provider_name, model)],
                };
                if let Some(ref reason) = judgment.reasoning {
                    segment.explanations.push(reason.clone());
                }
            }
            Ok(Err(e)) => {
                warn!("LLM analysis failed for segment {}: {}", idx, e);
            }
            Err(_) => {
                warn!("LLM analysis timeout for segment {}", idx);
            }
        }

        segments.push(segment);
    }

    segments
}
