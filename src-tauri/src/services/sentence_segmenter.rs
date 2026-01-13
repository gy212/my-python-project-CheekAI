// Text Segmenter Service Client
// 调用 Python 分句/分段服务 (spaCy + wtpsplit)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

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
    async fn test_paragraph_fallback_when_service_unavailable() {
        let text = "第一段落内容。\n\n第二段落内容。";
        let blocks = build_paragraph_blocks_wtp(text, "zh", 0.5, 100, 500, 1000).await;
        // 即使服务不可用，也应该返回结果（回退到本地）
        assert!(!blocks.is_empty());
    }
}
