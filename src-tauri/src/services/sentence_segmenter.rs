// Text Segmenter Service Client
// 调用 Python 分句/分段服务 (spaCy + wtpsplit)

use crate::services::providers::{get_api_key, parse_provider, ProviderClient, OPENAI_DEFAULT_MODEL};
use crate::services::ConfigStore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, warn};

const SENTENCE_REFINE_MAX_PAIRS: usize = 80;
const SENTENCE_REFINE_MAX_CALLS: usize = 3;
const SENTENCE_REFINE_MAX_SNIPPET_CHARS: usize = 240;

const SENTENCE_REFINE_SYSTEM_PROMPT: &str = r#"你是一个专业的中文/英文分句纠错器。
我会给你一组“候选切分边界”，每个边界由相邻两句组成（left/right）。
你的任务：判断该边界是否应该被“合并”（merge=true 表示这两句本应是一句，不该在这里断开）。

要求：
1) 不要改写任何文本，只做 merge 判定。
2) 保守策略：只有在非常确定是误切时才 merge=true。
3) 只输出 JSON，格式如下：
{"mergeIndices":[0,3,5]}
其中 mergeIndices 是需要合并的边界 index 列表（index 来自输入）。"#;

#[derive(Debug, Clone)]
struct SentenceSpan {
    start: i32,
    end: i32,
}

impl SentenceSpan {
    fn as_usize(&self) -> Option<(usize, usize)> {
        if self.start < 0 || self.end < 0 || self.end < self.start {
            return None;
        }
        let s = self.start as usize;
        let e = self.end as usize;
        Some((s, e))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundaryPair {
    index: usize,
    left: String,
    right: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefineResponse {
    #[serde(default)]
    merge_indices: Vec<usize>,
}

fn env_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn should_disable_llm_refine() -> bool {
    env_truthy("CHEEKAI_DISABLE_SENTENCE_LLM_REFINE")
}

fn head_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    s.chars().take(max_chars).collect()
}

fn tail_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}

fn extract_json(content: &str) -> String {
    let trimmed = content.trim();
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}

fn slice_by_bytes(text: &str, start: i32, end: i32) -> String {
    if start < 0 || end <= start {
        return String::new();
    }
    let len = text.len();
    let mut s = (start as usize).min(len);
    let mut e = (end as usize).min(len);
    if s >= e {
        return String::new();
    }
    while s < e && !text.is_char_boundary(s) {
        s += 1;
    }
    while e > s && !text.is_char_boundary(e) {
        e -= 1;
    }
    text.get(s..e).unwrap_or("").to_string()
}

fn ends_with_sentence_punct(s: &str) -> bool {
    let t = s.trim_end();
    t.chars()
        .last()
        .map(|c| matches!(c, '。' | '！' | '？' | '.' | '!' | '?' | '…'))
        .unwrap_or(false)
}

fn is_ambiguous_boundary(left: &str, right: &str) -> bool {
    let l = left.trim();
    let r = right.trim();
    if l.is_empty() || r.is_empty() {
        return false;
    }

    if !ends_with_sentence_punct(l) {
        return true;
    }

    // Unbalanced quotes/brackets often indicate the split is wrong.
    let quote_chars: &[char] = &['"', '“', '”', '\'', '‘', '’'];
    let l_quotes = l.chars().filter(|c| quote_chars.contains(c)).count();
    let r_quotes = r.chars().filter(|c| quote_chars.contains(c)).count();
    if (l_quotes % 2 == 1) || (r_quotes % 2 == 1) {
        return true;
    }

    let l_paren = l.chars().filter(|c| *c == '(').count() as i32
        - l.chars().filter(|c| *c == ')').count() as i32;
    if l_paren > 0 {
        return true;
    }

    // Dot-ending abbreviations and initials in English.
    if l.ends_with('.') {
        let lower_tail = l.to_ascii_lowercase();
        for abbr in [
            "e.g.", "i.e.", "etc.", "vs.", "mr.", "mrs.", "ms.", "dr.", "prof.", "fig.", "eq.",
            "no.", "inc.", "ltd.",
        ] {
            if lower_tail.ends_with(abbr) {
                return true;
            }
        }
        if l.len() >= 2 {
            let tail: String = l.chars().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
            if tail.chars().nth(0).map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                && tail.chars().nth(1) == Some('.')
            {
                return true;
            }
        }
        // If next sentence starts with lowercase, boundary is suspicious.
        if r.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
            return true;
        }
    }

    false
}

fn merge_spans_by_indices(spans: &[SentenceSpan], merge_indices: &[usize]) -> Vec<SentenceSpan> {
    if spans.is_empty() {
        return Vec::new();
    }
    let mut marks = std::collections::HashSet::new();
    for &idx in merge_indices {
        marks.insert(idx);
    }

    let mut out: Vec<SentenceSpan> = Vec::new();
    let mut i = 0usize;
    while i < spans.len() {
        let start = spans[i].start;
        let mut end = spans[i].end;
        let mut j = i;
        while j + 1 < spans.len() && marks.contains(&j) {
            end = spans[j + 1].end;
            j += 1;
        }
        out.push(SentenceSpan { start, end });
        i = j + 1;
    }
    out
}

fn aggregate_sentence_spans_to_blocks_with_breaks(
    text: &str,
    spans: &[SentenceSpan],
    target_chars: usize,
    max_chars: usize,
    hard_break_after: Option<&std::collections::HashSet<usize>>,
) -> Vec<crate::services::text_processor::TextBlock> {
    if spans.is_empty() {
        return vec![];
    }

    let mut blocks: Vec<crate::services::text_processor::TextBlock> = Vec::new();
    let mut current: Vec<&SentenceSpan> = Vec::new();
    let mut current_chars: usize = 0;

    for (idx, span) in spans.iter().enumerate() {
        // Enforce paragraph / section boundary: never merge sentences across this boundary.
        if !current.is_empty() {
            if let Some(breaks) = hard_break_after {
                if breaks.contains(&(idx.saturating_sub(1))) {
                    let start = current[0].start;
                    let end = current.last().unwrap().end;
                    let block_text = slice_by_bytes(text, start, end);
                    blocks.push(crate::services::text_processor::TextBlock {
                        index: blocks.len() as i32,
                        label: "sentence_block".to_string(),
                        need_detect: true,
                        merge_with_prev: false,
                        start,
                        end,
                        text: block_text,
                        sentence_count: Some(current.len() as i32),
                    });
                    current.clear();
                    current_chars = 0;
                }
            }
        }

        let sent_text = slice_by_bytes(text, span.start, span.end);
        let sent_len = sent_text.chars().count();
        if sent_len == 0 {
            continue;
        }

        if sent_len > max_chars {
            if !current.is_empty() {
                let start = current[0].start;
                let end = current.last().unwrap().end;
                let block_text = slice_by_bytes(text, start, end);
                blocks.push(crate::services::text_processor::TextBlock {
                    index: blocks.len() as i32,
                    label: "sentence_block".to_string(),
                    need_detect: true,
                    merge_with_prev: false,
                    start,
                    end,
                    text: block_text,
                    sentence_count: Some(current.len() as i32),
                });
                current.clear();
                current_chars = 0;
            }

            blocks.push(crate::services::text_processor::TextBlock {
                index: blocks.len() as i32,
                label: "sentence_block".to_string(),
                need_detect: true,
                merge_with_prev: false,
                start: span.start,
                end: span.end,
                text: sent_text,
                sentence_count: Some(1),
            });
            continue;
        }

        if current_chars + sent_len <= target_chars || current.is_empty() {
            current.push(span);
            current_chars += sent_len;
        } else {
            let start = current[0].start;
            let end = current.last().unwrap().end;
            let block_text = slice_by_bytes(text, start, end);
            blocks.push(crate::services::text_processor::TextBlock {
                index: blocks.len() as i32,
                label: "sentence_block".to_string(),
                need_detect: true,
                merge_with_prev: false,
                start,
                end,
                text: block_text,
                sentence_count: Some(current.len() as i32),
            });
            current = vec![span];
            current_chars = sent_len;
        }
    }

    if !current.is_empty() {
        let start = current[0].start;
        let end = current.last().unwrap().end;
        let block_text = slice_by_bytes(text, start, end);
        blocks.push(crate::services::text_processor::TextBlock {
            index: blocks.len() as i32,
            label: "sentence_block".to_string(),
            need_detect: true,
            merge_with_prev: false,
            start,
            end,
            text: block_text,
            sentence_count: Some(current.len() as i32),
        });
    }

    blocks
}

fn aggregate_sentence_spans_to_blocks(
    text: &str,
    spans: &[SentenceSpan],
    target_chars: usize,
    max_chars: usize,
) -> Vec<crate::services::text_processor::TextBlock> {
    aggregate_sentence_spans_to_blocks_with_breaks(text, spans, target_chars, max_chars, None)
}

fn select_provider_for_sentence_refine(
    provider: Option<&str>,
) -> Option<(String, String, String)> {
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

async fn call_llm_refine_boundaries(
    client: &ProviderClient,
    provider_name: &str,
    model: &str,
    api_key: &str,
    custom_url: Option<&str>,
    pairs: &[BoundaryPair],
) -> Result<Vec<usize>, String> {
    let payload = serde_json::to_string(pairs).map_err(|e| e.to_string())?;
    let user_prompt = format!(
        "候选边界列表（JSON数组，每项含 index/left/right）：\n{}\n\n请输出 mergeIndices。",
        payload
    );

    let max_tokens = 2048;
    let resp = match provider_name {
        "glm" => client
            .call_glm_with_url(custom_url, model, api_key, SENTENCE_REFINE_SYSTEM_PROMPT, &user_prompt, max_tokens, false)
            .await
            .map_err(|e| e.to_string())?,
        "deepseek" => client
            .call_deepseek_json_with_url(custom_url, model, api_key, SENTENCE_REFINE_SYSTEM_PROMPT, &user_prompt, max_tokens)
            .await
            .map_err(|e| e.to_string())?,
        "openai" => {
            let combined = format!("{}\n\n{}", SENTENCE_REFINE_SYSTEM_PROMPT, user_prompt);
            client
                .call_openai_responses_with_url(custom_url, model, api_key, &combined)
                .await
                .map_err(|e| e.to_string())?
        }
        "gemini" => client
            .call_gemini_with_url(custom_url, model, api_key, SENTENCE_REFINE_SYSTEM_PROMPT, &user_prompt, max_tokens)
            .await
            .map_err(|e| e.to_string())?,
        "anthropic" | "claude" => client
            .call_anthropic_with_url(custom_url, model, api_key, SENTENCE_REFINE_SYSTEM_PROMPT, &user_prompt, max_tokens)
            .await
            .map_err(|e| e.to_string())?,
        _ => {
            let combined = format!("{}\n\n{}", SENTENCE_REFINE_SYSTEM_PROMPT, user_prompt);
            client
                .call_openai_responses_with_url(custom_url, model, api_key, &combined)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    let json = extract_json(&resp.content);
    let parsed: RefineResponse = serde_json::from_str(&json)
        .map_err(|e| format!("refine json parse failed: {} content={}", e, json))?;
    Ok(parsed.merge_indices)
}

async fn segment_sentence_spans_best_effort(text: &str, language: &str) -> Vec<SentenceSpan> {
    let client = TextSegmenterClient::default();
    match client.segment_sentences(text, language).await {
        Ok(sentences) => sentences
            .into_iter()
            .map(|s| SentenceSpan { start: s.start, end: s.end })
            .collect(),
        Err(_) => crate::services::text_processor::split_sentences_advanced(text)
            .into_iter()
            .map(|s| SentenceSpan { start: s.start, end: s.end })
            .collect(),
    }
}

async fn refine_sentence_spans_with_llm(
    text: &str,
    spans: &[SentenceSpan],
    provider: Option<&str>,
    blocked_boundaries: Option<&std::collections::HashSet<usize>>,
) -> Vec<SentenceSpan> {
    if spans.len() < 2 {
        return spans.to_vec();
    }
    if should_disable_llm_refine() {
        return spans.to_vec();
    }

    let Some((provider_name, model, api_key)) = select_provider_for_sentence_refine(provider) else {
        return spans.to_vec();
    };

    let custom_url = resolve_custom_url(&provider_name);
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

    let mut candidates: Vec<BoundaryPair> = Vec::new();
    for i in 0..spans.len().saturating_sub(1) {
        if let Some(blocked) = blocked_boundaries {
            if blocked.contains(&i) {
                continue;
            }
        }
        let left_full = slice_by_bytes(text, spans[i].start, spans[i].end);
        let right_full = slice_by_bytes(text, spans[i + 1].start, spans[i + 1].end);
        if !is_ambiguous_boundary(&left_full, &right_full) {
            continue;
        }
        let left = tail_chars(left_full.trim(), SENTENCE_REFINE_MAX_SNIPPET_CHARS);
        let right = head_chars(right_full.trim(), SENTENCE_REFINE_MAX_SNIPPET_CHARS);
        if left.is_empty() || right.is_empty() {
            continue;
        }
        candidates.push(BoundaryPair { index: i, left, right });
    }

    if candidates.is_empty() {
        return spans.to_vec();
    }

    let mut all_merge_indices: Vec<usize> = Vec::new();
    let mut remaining = candidates;
    let mut calls = 0usize;
    while !remaining.is_empty() && calls < SENTENCE_REFINE_MAX_CALLS {
        calls += 1;
        let take = remaining.len().min(SENTENCE_REFINE_MAX_PAIRS);
        let batch: Vec<_> = remaining.drain(0..take).collect();
        match call_llm_refine_boundaries(
            &client,
            &provider_name,
            &model,
            &api_key,
            custom_url.as_deref(),
            &batch,
        )
        .await
        {
            Ok(mut merge_indices) => {
                // Validate indices are in the batch.
                let batch_set: std::collections::HashSet<usize> =
                    batch.iter().map(|p| p.index).collect();
                merge_indices.retain(|i| batch_set.contains(i));
                all_merge_indices.extend(merge_indices);
            }
            Err(e) => {
                warn!("[segmenter] sentence boundary refine failed: {}", e);
                break;
            }
        }
    }

    if !all_merge_indices.is_empty() {
        info!(
            "[segmenter] refined sentence boundaries using LLM: merges={} provider={} model={}",
            all_merge_indices.len(),
            provider_name,
            model
        );
    }

    merge_spans_by_indices(spans, &all_merge_indices)
}

/// 分句服务地址
fn usize_to_i32(value: usize) -> Option<i32> {
    if value <= i32::MAX as usize {
        Some(value as i32)
    } else {
        None
    }
}

fn char_offset_to_utf8_byte_index(text: &str, char_offset: usize) -> Option<usize> {
    if char_offset == 0 {
        return Some(0);
    }

    let mut current = 0usize;
    for (byte_idx, _) in text.char_indices() {
        if current == char_offset {
            return Some(byte_idx);
        }
        current += 1;
    }

    if current == char_offset {
        Some(text.len())
    } else {
        None
    }
}

fn normalize_offsets_to_utf8_bytes(text: &str, start: i32, end: i32) -> Option<(i32, i32)> {
    if start < 0 || end < 0 || end < start {
        return None;
    }

    let start_u = start as usize;
    let end_u = end as usize;
    let len_bytes = text.len();

    if start_u <= len_bytes
        && end_u <= len_bytes
        && text.is_char_boundary(start_u)
        && text.is_char_boundary(end_u)
    {
        return Some((start, end));
    }

    let start_b = char_offset_to_utf8_byte_index(text, start_u)?;
    let end_b = char_offset_to_utf8_byte_index(text, end_u)?;
    Some((usize_to_i32(start_b)?, usize_to_i32(end_b)?))
}

/// Segmenter service URL
const DEFAULT_SEGMENTER_URL: &str = "http://127.0.0.1:8788";

/// HTTP 客户端单例
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client")
    })
}

/// 分句结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceResult {
    pub text: String,
    pub start: i32,
    pub end: i32,
}

/// 分句块结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentenceBlock {
    pub index: i32,
    pub label: String,
    pub need_detect: bool,
    pub merge_with_prev: bool,
    pub start: i32,
    pub end: i32,
    pub text: String,
    pub sentence_count: i32,
}

/// 分段结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphResult {
    pub text: String,
    pub start: i32,
    pub end: i32,
}

/// 分段块结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphBlock {
    pub index: i32,
    pub label: String,
    pub need_detect: bool,
    pub merge_with_prev: bool,
    pub start: i32,
    pub end: i32,
    pub text: String,
    pub paragraph_count: i32,
}

/// 分句请求
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SegmentRequest {
    text: String,
    language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_chars: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_chars: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_chars: Option<i32>,
}

/// 分段请求
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParagraphRequest {
    text: String,
    language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_chars: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_chars: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_chars: Option<i32>,
}

/// 分句响应
#[derive(Debug, Deserialize)]
struct SegmentResponse {
    sentences: Vec<SentenceResult>,
}

/// 分块响应
#[derive(Debug, Deserialize)]
struct BlocksResponse {
    blocks: Vec<SentenceBlock>,
}

/// 分段响应
#[derive(Debug, Deserialize)]
struct ParagraphsResponse {
    paragraphs: Vec<ParagraphResult>,
}

/// 分段块响应
#[derive(Debug, Deserialize)]
struct ParagraphBlocksResponse {
    blocks: Vec<ParagraphBlock>,
}

/// 健康检查响应
#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
}

/// 分句/分段服务客户端
pub struct TextSegmenterClient {
    base_url: String,
}

impl Default for TextSegmenterClient {
    fn default() -> Self {
        Self::new(DEFAULT_SEGMENTER_URL)
    }
}

impl TextSegmenterClient {
    /// 创建新的客户端
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// 检查服务是否可用
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match get_client().get(&url).send().await {
            Ok(resp) => {
                if let Ok(health) = resp.json::<HealthResponse>().await {
                    health.status == "ok"
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// 使用 spaCy 进行智能分句
    pub async fn segment_sentences(
        &self,
        text: &str,
        language: &str,
    ) -> Result<Vec<SentenceResult>, String> {
        let url = format!("{}/segment", self.base_url);
        let request = SegmentRequest {
            text: text.to_string(),
            language: language.to_string(),
            min_chars: None,
            target_chars: None,
            max_chars: None,
        };

        let response = get_client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to call segmenter service: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Segmenter service returned error: {}",
                response.status()
            ));
        }

        let result: SegmentResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut sentences = Vec::with_capacity(result.sentences.len());
        for mut sent in result.sentences {
            let (start, end) = normalize_offsets_to_utf8_bytes(text, sent.start, sent.end)
                .ok_or_else(|| format!("Invalid sentence offsets: start={} end={}", sent.start, sent.end))?;
            sent.start = start;
            sent.end = end;
            sentences.push(sent);
        }

        Ok(sentences)
    }

    /// 使用 spaCy 分句并聚合为检测块
    pub async fn segment_to_blocks(
        &self,
        text: &str,
        language: &str,
        min_chars: i32,
        target_chars: i32,
        max_chars: i32,
    ) -> Result<Vec<SentenceBlock>, String> {
        let url = format!("{}/segment/blocks", self.base_url);
        let request = SegmentRequest {
            text: text.to_string(),
            language: language.to_string(),
            min_chars: Some(min_chars),
            target_chars: Some(target_chars),
            max_chars: Some(max_chars),
        };

        let response = get_client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to call segmenter service: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Segmenter service returned error: {}",
                response.status()
            ));
        }

        let result: BlocksResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut blocks = Vec::with_capacity(result.blocks.len());
        for mut block in result.blocks {
            let (start, end) = normalize_offsets_to_utf8_bytes(text, block.start, block.end).ok_or_else(
                || format!("Invalid sentence block offsets: start={} end={}", block.start, block.end),
            )?;
            block.start = start;
            block.end = end;
            blocks.push(block);
        }

        Ok(blocks)
    }

    /// 使用 wtpsplit 进行智能分段（话题分割）
    pub async fn segment_paragraphs(
        &self,
        text: &str,
        language: &str,
        threshold: f64,
    ) -> Result<Vec<ParagraphResult>, String> {
        let url = format!("{}/paragraph", self.base_url);
        let request = ParagraphRequest {
            text: text.to_string(),
            language: language.to_string(),
            threshold: Some(threshold),
            min_chars: None,
            target_chars: None,
            max_chars: None,
        };

        let response = get_client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to call segmenter service: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Segmenter service returned error: {}",
                response.status()
            ));
        }

        let result: ParagraphsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut paragraphs = Vec::with_capacity(result.paragraphs.len());
        for mut para in result.paragraphs {
            let (start, end) = normalize_offsets_to_utf8_bytes(text, para.start, para.end)
                .ok_or_else(|| format!("Invalid paragraph offsets: start={} end={}", para.start, para.end))?;
            para.start = start;
            para.end = end;
            paragraphs.push(para);
        }

        Ok(paragraphs)
    }

    /// 使用 wtpsplit 分段并聚合为检测块
    pub async fn segment_paragraphs_to_blocks(
        &self,
        text: &str,
        language: &str,
        threshold: f64,
        min_chars: i32,
        target_chars: i32,
        max_chars: i32,
    ) -> Result<Vec<ParagraphBlock>, String> {
        let url = format!("{}/paragraph/blocks", self.base_url);
        let request = ParagraphRequest {
            text: text.to_string(),
            language: language.to_string(),
            threshold: Some(threshold),
            min_chars: Some(min_chars),
            target_chars: Some(target_chars),
            max_chars: Some(max_chars),
        };

        let response = get_client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to call segmenter service: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Segmenter service returned error: {}",
                response.status()
            ));
        }

        let result: ParagraphBlocksResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut blocks = Vec::with_capacity(result.blocks.len());
        for mut block in result.blocks {
            let (start, end) = normalize_offsets_to_utf8_bytes(text, block.start, block.end).ok_or_else(
                || format!("Invalid paragraph block offsets: start={} end={}", block.start, block.end),
            )?;
            block.start = start;
            block.end = end;
            blocks.push(block);
        }

        Ok(blocks)
    }
}

/// 分句服务客户端 (别名，保持向后兼容)
pub type SentenceSegmenterClient = TextSegmenterClient;

/// 将 ParagraphBlock 转换为 TextBlock 格式
impl From<ParagraphBlock> for crate::services::text_processor::TextBlock {
    fn from(block: ParagraphBlock) -> Self {
        Self {
            index: block.index,
            label: block.label,
            need_detect: block.need_detect,
            merge_with_prev: block.merge_with_prev,
            start: block.start,
            end: block.end,
            text: block.text,
            sentence_count: Some(block.paragraph_count),
        }
    }
}

/// 将 SentenceBlock 转换为 TextBlock 格式
impl From<SentenceBlock> for crate::services::text_processor::TextBlock {
    fn from(block: SentenceBlock) -> Self {
        Self {
            index: block.index,
            label: block.label,
            need_detect: block.need_detect,
            merge_with_prev: block.merge_with_prev,
            start: block.start,
            end: block.end,
            text: block.text,
            sentence_count: Some(block.sentence_count),
        }
    }
}

/// 使用 spaCy 服务构建句子块（带回退）
/// 如果 spaCy 服务不可用，回退到本地规则分句
pub async fn build_sentence_blocks_spacy(
    text: &str,
    language: &str,
    min_chars: i32,
    target_chars: i32,
    max_chars: i32,
) -> Vec<crate::services::text_processor::TextBlock> {
    let client = TextSegmenterClient::default();
    
    // 尝试使用 spaCy 服务
    match client
        .segment_to_blocks(text, language, min_chars, target_chars, max_chars)
        .await
    {
        Ok(blocks) => {
            eprintln!("[segmenter] Using spaCy service, got {} blocks", blocks.len());
            blocks.into_iter().map(|b| b.into()).collect()
        }
        Err(e) => {
            eprintln!("[segmenter] spaCy service unavailable ({}), falling back to local", e);
            // 回退到本地分句
            crate::services::text_processor::build_sentence_blocks(
                text,
                min_chars as usize,
                target_chars as usize,
                max_chars as usize,
            )
        }
    }
}

/// 智能分句（spaCy -> 规则 -> LLM 归整）并聚合为句子块
///
/// - 优先使用 spaCy 服务得到句子 offsets（UTF-8 byte offsets）
/// - 如果服务不可用，回退到本地 `split_sentences_advanced`
/// - 对可能误切的边界，使用 LLM 做“合并裁决”（失败则跳过）
/// - 最后按 target/max 进行合块，保证不会“截断”任何 UTF-8 字符
pub async fn build_sentence_blocks_smart(
    text: &str,
    language: &str,
    min_chars: i32,
    target_chars: i32,
    max_chars: i32,
    provider: Option<&str>,
) -> Vec<crate::services::text_processor::TextBlock> {
    let client = TextSegmenterClient::default();

    let mut spans: Vec<SentenceSpan> = match client.segment_sentences(text, language).await {
        Ok(sentences) => {
            info!(
                "[segmenter] Using spaCy service, got {} sentences",
                sentences.len()
            );
            sentences
                .into_iter()
                .map(|s| SentenceSpan { start: s.start, end: s.end })
                .collect()
        }
        Err(e) => {
            warn!(
                "[segmenter] spaCy service unavailable ({}), falling back to local",
                e
            );
            crate::services::text_processor::split_sentences_advanced(text)
                .into_iter()
                .map(|s| SentenceSpan { start: s.start, end: s.end })
                .collect()
        }
    };

    spans.retain(|s| s.as_usize().is_some());
    if spans.is_empty() {
        return crate::services::text_processor::build_sentence_blocks(
            text,
            min_chars as usize,
            target_chars as usize,
            max_chars as usize,
        );
    }

    let refined = refine_sentence_spans_with_llm(text, &spans, provider, None).await;
    spans = refined;

    aggregate_sentence_spans_to_blocks(text, &spans, target_chars as usize, max_chars as usize)
}

/// 在指定段落块范围内构建句子块（不会跨段落合并），用于减少非正文内容带来的检测开销。
pub async fn build_sentence_blocks_smart_in_paragraphs(
    text: &str,
    language: &str,
    paragraphs: &[crate::services::text_processor::TextBlock],
    target_chars: i32,
    max_chars: i32,
    provider: Option<&str>,
) -> Vec<crate::services::text_processor::TextBlock> {
    let mut spans: Vec<SentenceSpan> = Vec::new();
    let mut hard_break_after: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut prev_kept_end: Option<i32> = None;

    for para in paragraphs.iter().filter(|b| b.need_detect) {
        let para_text = slice_by_bytes(text, para.start, para.end);
        if para_text.trim().is_empty() {
            continue;
        }

        let before_len = spans.len();
        let mut local_spans = segment_sentence_spans_best_effort(&para_text, language).await;
        local_spans.retain(|s| s.as_usize().is_some());
        if local_spans.is_empty() {
            continue;
        }

        // Prevent blocks/LLM-merge from spanning across regions that contain filtered-out content.
        // If the gap between the last kept paragraph end and the next paragraph start contains any
        // non-whitespace, a cross-paragraph block would re-introduce those skipped chars (since we
        // slice the original text by [start..end]).
        if before_len > 0 {
            if let Some(prev_end) = prev_kept_end {
                let gap = slice_by_bytes(text, prev_end, para.start);
                if gap.chars().any(|c| !c.is_whitespace()) {
                    hard_break_after.insert(before_len - 1);
                }
            }
        }

        for mut s in local_spans {
            s.start += para.start;
            s.end += para.start;
            spans.push(s);
        }
        prev_kept_end = Some(para.end);
    }

    if spans.is_empty() {
        return vec![];
    }

    let spans = refine_sentence_spans_with_llm(text, &spans, provider, Some(&hard_break_after)).await;
    aggregate_sentence_spans_to_blocks_with_breaks(
        text,
        &spans,
        target_chars as usize,
        max_chars as usize,
        Some(&hard_break_after),
    )
}

/// 智能分句：返回“句子列表”（UTF-8 byte offsets），可用于调试/可视化。
///
/// 注意：LLM 归整只会“合并边界”，不会重写文本，因此 offsets 总能对齐原文。
pub async fn segment_sentences_smart(
    text: &str,
    language: &str,
    provider: Option<&str>,
) -> Vec<SentenceResult> {
    let client = TextSegmenterClient::default();

    let mut spans: Vec<SentenceSpan> = match client.segment_sentences(text, language).await {
        Ok(sentences) => sentences
            .into_iter()
            .map(|s| SentenceSpan { start: s.start, end: s.end })
            .collect(),
        Err(_) => crate::services::text_processor::split_sentences_advanced(text)
            .into_iter()
            .map(|s| SentenceSpan { start: s.start, end: s.end })
            .collect(),
    };

    spans.retain(|s| s.as_usize().is_some());
    if spans.is_empty() {
        return vec![];
    }

    spans = refine_sentence_spans_with_llm(text, &spans, provider, None).await;

    spans
        .into_iter()
        .map(|s| SentenceResult {
            start: s.start,
            end: s.end,
            text: slice_by_bytes(text, s.start, s.end),
        })
        .filter(|s| !s.text.trim().is_empty())
        .collect()
}

/// 使用 wtpsplit 服务构建段落块（带回退）
/// 如果 wtpsplit 服务不可用，回退到本地规则分段
pub async fn build_paragraph_blocks_wtp(
    text: &str,
    language: &str,
    threshold: f64,
    min_chars: i32,
    target_chars: i32,
    max_chars: i32,
) -> Vec<crate::services::text_processor::TextBlock> {
    let client = TextSegmenterClient::default();
    
    // 尝试使用 wtpsplit 服务
    match client
        .segment_paragraphs_to_blocks(text, language, threshold, min_chars, target_chars, max_chars)
        .await
    {
        Ok(blocks) => {
            eprintln!("[segmenter] Using wtpsplit service, got {} paragraph blocks", blocks.len());
            blocks.into_iter().map(|b| b.into()).collect()
        }
        Err(e) => {
            eprintln!("[segmenter] wtpsplit service unavailable ({}), falling back to local", e);
            // 回退到本地分段（简单空行分割）
            crate::services::text_processor::build_paragraph_blocks(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_spans_by_indices_chain() {
        let spans = vec![
            SentenceSpan { start: 0, end: 10 },
            SentenceSpan { start: 10, end: 20 },
            SentenceSpan { start: 20, end: 30 },
            SentenceSpan { start: 30, end: 40 },
        ];

        let merged = merge_spans_by_indices(&spans, &[1, 2]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].start, 0);
        assert_eq!(merged[0].end, 10);
        assert_eq!(merged[1].start, 10);
        assert_eq!(merged[1].end, 40);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = TextSegmenterClient::default();
        assert_eq!(client.base_url, DEFAULT_SEGMENTER_URL);
    }

    #[tokio::test]
    async fn test_sentence_fallback_when_service_unavailable() {
        let text = "这是第一句。这是第二句！这是第三句？";
        let blocks = build_sentence_blocks_spacy(text, "zh", 50, 200, 300).await;
        // 即使服务不可用，也应该返回结果（回退到本地）
        assert!(!blocks.is_empty());
    }

    #[tokio::test]
    async fn test_sentence_smart_fallback_when_service_unavailable() {
        let text = "这是第一句。这是第二句！这是第三句？";
        let blocks = build_sentence_blocks_smart(text, "zh", 50, 200, 300, None).await;
        assert!(!blocks.is_empty());
    }

    #[tokio::test]
    async fn test_sentence_smart_in_paragraphs_does_not_cross_boundary() {
        // Skip middle paragraph and ensure blocks don't span across it (would re-introduce skipped text).
        let text = "标题\n\n这是第一段。这里还有一句。\n\n目录.....1\n\n第二段开始。";
        let all = crate::services::text_processor::build_paragraph_blocks(text);
        assert!(all.len() >= 4);
        let paragraphs = vec![all[1].clone(), all[3].clone()];
        let blocks =
            build_sentence_blocks_smart_in_paragraphs(text, "zh", &paragraphs, 200, 300, None).await;
        assert!(!blocks.is_empty());
        for b in blocks {
            let block_text = slice_by_bytes(text, b.start, b.end);
            assert!(!block_text.contains("目录"));
        }
    }

    #[tokio::test]
    async fn test_paragraph_fallback_when_service_unavailable() {
        let text = "第一段落内容。\n\n第二段落内容。";
        let blocks = build_paragraph_blocks_wtp(text, "zh", 0.5, 100, 500, 1000).await;
        // 即使服务不可用，也应该返回结果（回退到本地）
        assert!(!blocks.is_empty());
    }
}
