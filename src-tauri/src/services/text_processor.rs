// Text Processing Service
// Migrated from Python service.py

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Normalize punctuation in text (Chinese/English)
pub fn normalize_punctuation(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut s = text.to_string();

    // Replace smart quotes
    s = s.replace('\u{201c}', "\"")  // "
         .replace('\u{201d}', "\"")  // "
         .replace('\u{2018}', "'")   // '
         .replace('\u{2019}', "'");  // '

    // Replace em dash
    s = s.replace('\u{2014}', "-");

    // Replace ideographic space and non-breaking space
    let space_re = Regex::new(r"[\u{3000}\u{00A0}]").unwrap();
    s = space_re.replace_all(&s, " ").to_string();

    // Normalize line endings
    s = s.replace("\r\n", "\n").replace('\r', "\n");

    // Collapse horizontal whitespace
    let ws_re = Regex::new(r"[ \t\x0C\x0B]+").unwrap();
    s = ws_re.replace_all(&s, " ").to_string();

    // Strip each line
    s = s.lines()
         .map(|ln| ln.trim())
         .collect::<Vec<_>>()
         .join("\n");

    s.trim().to_string()
}

/// Estimate token count (Chinese chars + English words)
pub fn estimate_tokens(text: &str) -> i32 {
    if text.is_empty() {
        return 1;
    }

    let re = Regex::new(r"[A-Za-z0-9_]+|[\u{4e00}-\u{9fff}]").unwrap();
    let count = re.find_iter(text).count();
    std::cmp::max(1, count as i32)
}

/// Simple sentence splitting
pub fn split_sentences(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    // Rust regex doesn't support lookbehind, use alternative approach
    let re = Regex::new(r"([。！？?!])\s+").unwrap();
    let result = re.replace_all(text, "$1\x00");
    result.split('\x00')
          .filter(|p| !p.is_empty())
          .map(|s| s.to_string())
          .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceOffset {
    pub text: String,
    pub start: i32,
    pub end: i32,
}

/// Advanced sentence splitting with offset tracking
pub fn split_sentences_advanced(text: &str) -> Vec<SentenceOffset> {
    if text.is_empty() {
        return vec![];
    }

    let mut sentences = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let quote_chars: HashSet<char> = ['"', '\u{201c}', '\u{201d}', '\'', '\u{2018}', '\u{2019}'].iter().cloned().collect();

    let mut current_start: usize = 0;
    let mut buffer = String::new();
    let mut in_quote = false;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        buffer.push(ch);

        // Track quote state
        if quote_chars.contains(&ch) {
            in_quote = !in_quote;
        }

        // Check for sentence ending
        let mut is_sentence_end = false;
        if ['。', '！', '？', '.', '!', '?'].contains(&ch) {
            // Don't split inside quotes
            if in_quote {
                i += 1;
                continue;
            }

            // Check for decimal numbers
            if ch == '.' && i > 0 && i < chars.len() - 1 {
                if chars[i - 1].is_ascii_digit() && chars[i + 1].is_ascii_digit() {
                    i += 1;
                    continue;
                }
            }

            is_sentence_end = true;
        }

        if is_sentence_end {
            // Skip trailing whitespace
            while i < chars.len() - 1 && [' ', '\t'].contains(&chars[i + 1]) {
                i += 1;
                buffer.push(chars[i]);
            }

            let sentence_text = buffer.trim().to_string();
            if !sentence_text.is_empty() {
                sentences.push(SentenceOffset {
                    text: sentence_text,
                    start: current_start as i32,
                    end: (current_start + buffer.len()) as i32,
                });
                current_start += buffer.len();
                buffer.clear();
            }
        }

        i += 1;
    }

    // Handle remaining buffer
    let remaining = buffer.trim().to_string();
    if !remaining.is_empty() {
        sentences.push(SentenceOffset {
            text: remaining,
            start: current_start as i32,
            end: text.len() as i32,
        });
    }

    sentences
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextBlock {
    pub index: i32,
    pub label: String,
    pub need_detect: bool,
    pub merge_with_prev: bool,
    pub start: i32,
    pub end: i32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_count: Option<i32>,
}

/// Build paragraph blocks from plain text
/// Uses blank lines (one or more consecutive empty lines) as paragraph separators
/// to preserve the original document structure.
pub fn build_paragraph_blocks(text: &str) -> Vec<TextBlock> {
    let mut blocks = Vec::new();

    if text.is_empty() {
        return blocks;
    }

    // Split on blank lines (one or more consecutive empty lines)
    let para_re = Regex::new(r"\n\s*\n").unwrap();
    let mut cursor: usize = 0;

    for para in para_re.split(text) {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            cursor += para.len() + 2;
            continue;
        }

        // Find actual start position in original text
        let start = text[cursor..].find(trimmed).map(|i| cursor + i).unwrap_or(cursor);
        let end = start + trimmed.len();

        blocks.push(TextBlock {
            index: blocks.len() as i32,
            label: "body".to_string(),
            need_detect: true,
            merge_with_prev: false,
            start: start as i32,
            end: end as i32,
            text: trimmed.to_string(),
            sentence_count: None,
        });

        cursor = end;
    }

    // Ensure at least one block
    if blocks.is_empty() {
        blocks.push(TextBlock {
            index: 0,
            label: "body".to_string(),
            need_detect: true,
            merge_with_prev: false,
            start: 0,
            end: text.len() as i32,
            text: text.trim().to_string(),
            sentence_count: None,
        });
    }

    blocks
}

#[allow(dead_code)]
fn postprocess_paragraph_blocks(blocks: Vec<TextBlock>, text: &str) -> Vec<TextBlock> {
    // If there's only one block, keep it to avoid dropping the entire content for short inputs.
    if blocks.len() <= 1 {
        return blocks;
    }

    let mut out: Vec<TextBlock> = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        let blk = &blocks[i];
        let blk_text = &text[blk.start as usize..blk.end as usize];

        if !is_short_title_like(blk_text) {
            out.push(blk.clone());
            i += 1;
            continue;
        }

        // Gather consecutive short-title-like blocks (title + subtitle, etc.).
        let seq_start = i;
        let mut j = i + 1;
        while j < blocks.len() {
            let t = &text[blocks[j].start as usize..blocks[j].end as usize];
            if is_short_title_like(t) {
                j += 1;
            } else {
                break;
            }
        }

        // Prefer merging into the next non-title block.
        if j < blocks.len() {
            let mut merged = blocks[j].clone();
            merged.start = blocks[seq_start].start;
            // Keep end as the body's end; the slice includes the skipped title(s) due to adjusted start.
            out.push(merged);
            i = j + 1;
            continue;
        }

        // No next body: merge into previous body if available, otherwise drop.
        if let Some(prev) = out.last_mut() {
            prev.end = blocks[j.saturating_sub(1)].end;
        }
        i = j;
    }

    if out.is_empty() {
        return blocks;
    }

    for (idx, b) in out.iter_mut().enumerate() {
        b.index = idx as i32;
    }

    out
}

fn is_short_title_like(s: &str) -> bool {
    let trimmed = s.trim();
    let non_ws = trimmed.chars().filter(|c| !c.is_whitespace()).count();
    if non_ws == 0 {
        return true;
    }
    if non_ws >= 20 {
        return false;
    }
    if has_sentence_end_punctuation(trimmed) {
        return false;
    }
    true
}

fn has_sentence_end_punctuation(s: &str) -> bool {
    s.contains('。') || s.contains('.') || s.contains('！') || s.contains('!') || s.contains('？') || s.contains('?')
}

/// Build sentence-based detection blocks
pub fn build_sentence_blocks(
    text: &str,
    _min_chars: usize,
    target_chars: usize,
    max_chars: usize,
) -> Vec<TextBlock> {
    let sentences = split_sentences_advanced(text);
    if sentences.is_empty() {
        return vec![];
    }

    let mut blocks = Vec::new();
    let mut current_sentences: Vec<&SentenceOffset> = Vec::new();
    let mut current_chars = 0;

    for sent in &sentences {
        // Use char count (not UTF-8 byte length) so thresholds behave consistently for CJK text.
        let sent_len = sent.text.chars().count();

        // Long sentence becomes standalone block
        if sent_len > max_chars {
            // Flush current block
            if !current_sentences.is_empty() {
                let block_text = current_sentences.iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                blocks.push(TextBlock {
                    index: blocks.len() as i32,
                    label: "sentence_block".to_string(),
                    need_detect: true,
                    merge_with_prev: false,
                    start: current_sentences[0].start,
                    end: current_sentences.last().unwrap().end,
                    text: block_text,
                    sentence_count: Some(current_sentences.len() as i32),
                });
                current_sentences.clear();
                current_chars = 0;
            }

            blocks.push(TextBlock {
                index: blocks.len() as i32,
                label: "sentence_block".to_string(),
                need_detect: true,
                merge_with_prev: false,
                start: sent.start,
                end: sent.end,
                text: sent.text.clone(),
                sentence_count: Some(1),
            });
            continue;
        }

        // Try to add to current block
        if current_chars + sent_len <= target_chars || current_chars == 0 {
            current_sentences.push(sent);
            current_chars += sent_len;
        } else {
            // Flush current block
            let block_text = current_sentences.iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            blocks.push(TextBlock {
                index: blocks.len() as i32,
                label: "sentence_block".to_string(),
                need_detect: true,
                merge_with_prev: false,
                start: current_sentences[0].start,
                end: current_sentences.last().unwrap().end,
                text: block_text,
                sentence_count: Some(current_sentences.len() as i32),
            });

            current_sentences = vec![sent];
            current_chars = sent_len;
        }
    }

    // Flush remaining
    if !current_sentences.is_empty() {
        let block_text = current_sentences.iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        blocks.push(TextBlock {
            index: blocks.len() as i32,
            label: "sentence_block".to_string(),
            need_detect: true,
            merge_with_prev: false,
            start: current_sentences[0].start,
            end: current_sentences.last().unwrap().end,
            text: block_text,
            sentence_count: Some(current_sentences.len() as i32),
        });
    }

    blocks
}

/// Compute stylometry metrics for text
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StylometryMetrics {
    pub ttr: f64,              // Type-Token Ratio
    pub avg_sentence_len: f64,
    pub function_word_ratio: Option<f64>,
    pub repeat_ratio: Option<f64>,
    pub ngram_repeat_rate: Option<f64>,
    pub punctuation_ratio: Option<f64>,
}

pub fn compute_stylometry(text: &str) -> StylometryMetrics {
    if text.is_empty() {
        return StylometryMetrics::default();
    }

    let word_re = Regex::new(r"[A-Za-z0-9_]+|[\u{4e00}-\u{9fff}]").unwrap();
    let words: Vec<&str> = word_re.find_iter(text).map(|m| m.as_str()).collect();
    let total_words = words.len();

    if total_words == 0 {
        return StylometryMetrics::default();
    }

    // Type-Token Ratio
    let unique_words: HashSet<&str> = words.iter().cloned().collect();
    let ttr = unique_words.len() as f64 / total_words as f64;

    // Average sentence length (in chars); advanced splitter handles CJK without whitespace.
    let sentences = split_sentences_advanced(text);
    let avg_sentence_len = if sentences.is_empty() {
        text.chars().count() as f64
    } else {
        sentences.iter().map(|s| s.text.chars().count()).sum::<usize>() as f64 / sentences.len() as f64
    };

    // Function word ratio (small Chinese set, legacy-compatible)
    let function_words: HashSet<&str> = [
        "的", "之", "一", "是", "了", "在", "有", "和", "与", "这", "对", "也", "为", "而", "并且",
    ]
    .into_iter()
    .collect();
    let function_ratio = words.iter().filter(|t| function_words.contains(**t)).count() as f64
        / total_words.max(1) as f64;

    // Punctuation ratio (legacy-compatible punctuation set)
    let punct_re = Regex::new(r"[，。！？.!?]").unwrap();
    let punct_count = punct_re.find_iter(text).count();
    let char_len = text.chars().count().max(1) as f64;
    let punctuation_ratio = punct_count as f64 / char_len;

    // Repeat ratio: fraction of vocab items that occur >= 3 times
    let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in &words {
        *freq.entry(*t).or_insert(0) += 1;
    }
    let repeats = freq.values().filter(|&&v| v >= 3).count() as f64 / freq.len().max(1) as f64;

    // 3-gram repeat rate
    let ngram_rate = ngram_repeat_rate(&words, 3);

    StylometryMetrics {
        ttr,
        avg_sentence_len,
        function_word_ratio: Some(function_ratio),
        repeat_ratio: Some(repeats),
        ngram_repeat_rate: Some(ngram_rate),
        punctuation_ratio: Some(punctuation_ratio),
    }
}

fn ngram_repeat_rate(tokens: &[&str], n: usize) -> f64 {
    if n == 0 || tokens.len() < n + 1 {
        return 0.0;
    }
    let mut counts: std::collections::HashMap<Vec<&str>, usize> = std::collections::HashMap::new();
    let mut total = 0usize;
    for i in 0..=tokens.len().saturating_sub(n) {
        let key: Vec<&str> = tokens[i..i + n].to_vec();
        *counts.entry(key).or_insert(0) += 1;
        total += 1;
    }
    let repeats = counts.values().filter(|&&c| c >= 2).map(|&c| c - 1).sum::<usize>();
    repeats as f64 / total.max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_punctuation() {
        let input = "Hello\u{201c}World\u{201d}";
        let output = normalize_punctuation(input);
        assert_eq!(output, "Hello\"World\"");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("Hello World"), 2);
        assert_eq!(estimate_tokens("你好世界"), 4);
        assert_eq!(estimate_tokens("Hello 你好"), 3);
    }

    #[test]
    fn test_split_sentences() {
        let text = "这是第一句。这是第二句！这是第三句？";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 1); // No whitespace between, so no split
    }

    #[test]
    fn test_build_paragraph_blocks() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let blocks = build_paragraph_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_postprocess_merges_short_title_into_body() {
        let text = "My Title\n\nThis is the body paragraph.";
        let blocks = build_paragraph_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start, 0);
        assert_eq!(blocks[0].end, text.len() as i32);
    }

    #[test]
    fn test_postprocess_merges_multiple_title_lines_into_body() {
        let text = "Main Title\n\nSubtitle\n\nBody starts here with a sentence.";
        let blocks = build_paragraph_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start, 0);
        assert_eq!(blocks[0].end, text.len() as i32);
    }

    #[test]
    fn test_build_sentence_blocks_uses_char_count_for_cjk() {
        let sentence = format!("{}.", "\u{4e00}".repeat(25));
        let text = sentence.repeat(10);
        let blocks = build_sentence_blocks(&text, 50, 200, 300);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].sentence_count, Some(7));
        assert_eq!(blocks[1].sentence_count, Some(3));
    }
}
