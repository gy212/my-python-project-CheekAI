use cheek_ai_lib::api::preprocess_file;
use cheek_ai_lib::services::sentence_segmenter::{
    build_sentence_blocks_smart,
    build_sentence_blocks_smart_in_paragraphs,
    segment_sentences_smart,
};
use cheek_ai_lib::services::text_processor::normalize_punctuation;
use serde::Serialize;

fn detect_language_simple(text: &str) -> String {
    let chinese_count = text
        .chars()
        .filter(|c| *c >= '\u{4e00}' && *c <= '\u{9fff}')
        .count();
    let total_chars = text.chars().filter(|c| !c.is_whitespace()).count();

    if total_chars > 0 && chinese_count as f64 / total_chars as f64 > 0.3 {
        "zh".to_string()
    } else {
        "en".to_string()
    }
}

fn preview(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push_str("...");
    }
    out.replace('\n', " ")
}

fn parse_arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn has_flag(args: &[String], key: &str) -> bool {
    args.iter().any(|a| a == key)
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage:\n  cargo run -p cheekAI --bin segment_docx -- <path.docx> [--provider <name[:model]>] [--sentences <n>] [--blocks <n>] [--llm] [--filter] [--out <json_path>]\n\nNotes:\n  - 默认不开启 LLM 归整（除非指定 --llm）。\n  - `--filter` 会先做段落级内容过滤（规则 + LLM），再在正文段落范围内分句。\n  - 可用环境变量 CHEEKAI_DISABLE_SENTENCE_LLM_REFINE=1 强制关闭 LLM 归整。"
        );
        return Ok(());
    }

    let path = args[1].clone();
    let provider = parse_arg_value(&args, "--provider");
    let sentences_n: usize = parse_arg_value(&args, "--sentences")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let blocks_n: usize = parse_arg_value(&args, "--blocks")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let enable_llm = has_flag(&args, "--llm");
    let enable_filter = has_flag(&args, "--filter");
    let out_path = parse_arg_value(&args, "--out");

    let bytes = std::fs::read(&path).map_err(|e| format!("read file failed: {}", e))?;
    let file_name = std::path::Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "input.docx".to_string());

    let extracted = preprocess_file(file_name.clone(), bytes).await?;
    let text = normalize_punctuation(&extracted);
    let language = detect_language_simple(&text);

    println!("File: {}", path);
    println!("Extracted: {} chars ({} bytes)", text.chars().count(), text.len());
    println!("Language: {}", language);
    println!("Provider: {}", provider.clone().unwrap_or_else(|| "(auto)".to_string()));
    println!("LLM refine: {}", if enable_llm { "on" } else { "off" });
    println!("Paragraph filter: {}", if enable_filter { "on" } else { "off" });
    println!();

    let provider_arg = if enable_llm { provider.as_deref() } else { None };

    let mut filter_summary: Option<cheek_ai_lib::services::FilterSummary> = None;
    let blocks = if enable_filter {
        let paras = cheek_ai_lib::services::text_processor::build_paragraph_blocks(&text);
        let (body_paras, summary) = cheek_ai_lib::services::filter_paragraphs(&paras, provider_arg).await;
        filter_summary = Some(summary);
        build_sentence_blocks_smart_in_paragraphs(&text, &language, &body_paras, 200, 300, provider_arg).await
    } else {
        build_sentence_blocks_smart(&text, &language, 50, 200, 300, provider_arg).await
    };

    let sentences = if enable_filter {
        // For readability we dump sentence list only when not filtering; filtered path focuses on blocks.
        Vec::new()
    } else {
        segment_sentences_smart(&text, &language, provider_arg).await
    };

    if !sentences.is_empty() {
        println!("Sentences: {}", sentences.len());
        for (i, s) in sentences.iter().take(sentences_n).enumerate() {
            let len = s.text.chars().count();
            println!(
                "[S{:04}] bytes=[{},{}] chars={}  {}",
                i,
                s.start,
                s.end,
                len,
                preview(&s.text, 120)
            );
        }
        if sentences.len() > sentences_n {
            println!("... ({} more sentences)", sentences.len() - sentences_n);
        }
        println!();
    } else if enable_filter {
        if let Some(ref s) = filter_summary {
            println!(
                "Filter summary: total={} body={} filtered={} (rule={} llm={})",
                s.total_paragraphs,
                s.body_count,
                s.filtered_count,
                s.filtered_by_rule,
                s.filtered_by_llm
            );
            println!();
        }
    }

    println!("Sentence blocks: {}", blocks.len());
    for b in blocks.iter().take(blocks_n) {
        let len = b.text.chars().count();
        println!(
            "[B{:04}] bytes=[{},{}] chars={} sentences={:?}  {}",
            b.index,
            b.start,
            b.end,
            len,
            b.sentence_count,
            preview(&b.text, 140)
        );
    }
    if blocks.len() > blocks_n {
        println!("... ({} more blocks)", blocks.len() - blocks_n);
    }

    if let Some(out_path) = out_path {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Output {
            file: String,
            language: String,
            llm_refine: bool,
            filter: bool,
            provider: Option<String>,
            extracted_chars: usize,
            extracted_bytes: usize,
            sentences: Vec<cheek_ai_lib::services::sentence_segmenter::SentenceResult>,
            blocks: Vec<cheek_ai_lib::services::text_processor::TextBlock>,
            #[serde(skip_serializing_if = "Option::is_none")]
            filter_summary: Option<cheek_ai_lib::services::FilterSummary>,
        }

        let out = Output {
            file: path.clone(),
            language: language.clone(),
            llm_refine: enable_llm,
            filter: enable_filter,
            provider: provider.clone(),
            extracted_chars: text.chars().count(),
            extracted_bytes: text.len(),
            sentences,
            blocks,
            filter_summary,
        };

        let json = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
        std::fs::write(&out_path, json).map_err(|e| format!("write out failed: {}", e))?;
        println!();
        println!("Wrote JSON: {}", out_path);
    }

    Ok(())
}
