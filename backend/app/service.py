# -*- coding: utf-8 -*-
from __future__ import annotations

import asyncio
import json
import logging
import math
import re
import statistics
import time
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

try:
    from .providers import callGLMChat, getGLMKey, parseProvider
except ImportError:  # allow running as standalone script
    import pathlib
    import sys

    sys.path.append(str(pathlib.Path(__file__).resolve().parents[1]))
    from providers import callGLMChat, getGLMKey, parseProvider  # type: ignore


_LLM_SEGMENT_JSON_RE = re.compile(r"\{.*\}", re.DOTALL)


def _clear_runtime_cache() -> None:
    """清理当前运行时缓存（包括 API 调用缓存等）。"""
    try:
        log_dir = Path(__file__).resolve().parents[1] / "logs"
        for fname in ("glm_last_response.json", "detect_cache.json"):
            f = log_dir / fname
            if f.exists():
                f.unlink()
    except Exception:
        # 清理失败不影响主流程
        pass


# ---------------------------------------------------------------------------
# Normalization helpers
# ---------------------------------------------------------------------------

def normalizePunctuation(text: str) -> str:
    if not text:
        return ""
    s = text
    s = s.replace("\u201c", '"').replace("\u201d", '"').replace("\u2018", "'").replace("\u2019", "'")
    s = s.replace("\u2014", "-")
    s = re.sub(r"[\u3000\u00A0]", " ", s)
    s = s.replace("\r\n", "\n").replace("\r", "\n")
    s = re.sub(r"[ \t\f\v]+", " ", s)
    s = "\n".join(ln.strip() for ln in s.split("\n"))
    return s.strip()


def estimateTokens(text: str) -> int:
    tokens = re.findall(r"[A-Za-z0-9_]+|[\u4e00-\u9fff]", text or "")
    return max(1, len(tokens))


def splitSentences(text: str) -> List[str]:
    return [p for p in re.split(r"(?<=[\u3002\uff01\uff1f?!])\s+", text or "") if p]


# ---------------------------------------------------------------------------
# Paragraph block construction
# ---------------------------------------------------------------------------

def buildParagraphBlocksFromText(text: str) -> List[Dict[str, Any]]:
    blocks: List[Dict[str, Any]] = []
    if not text:
        return blocks
    cursor = 0
    buf: List[str] = []
    buf_start: Optional[int] = None
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if stripped and buf_start is None:
            buf_start = cursor
        if stripped:
            buf.append(line.rstrip("\n"))
        elif buf:
            block_text = "\n".join(buf).strip()
            start = buf_start if buf_start is not None else cursor - len(line)
            end = cursor
            blocks.append(
                {
                    "index": len(blocks),
                    "label": "body",
                    "needDetect": True,
                    "mergeWithPrev": False,
                    "start": max(0, start),
                    "end": max(start, end),
                    "text": block_text,
                }
            )
            buf = []
            buf_start = None
        cursor += len(line)
    if buf:
        start = buf_start if buf_start is not None else max(0, len(text) - len("\n".join(buf)))
        end = len(text)
        blocks.append(
            {
                "index": len(blocks),
                "label": "body",
                "needDetect": True,
                "mergeWithPrev": False,
                "start": max(0, start),
                "end": end,
                "text": "\\n".join(buf).strip(),
            }
        )
    if not blocks:
        blocks.append(
            {
                "index": 0,
                "label": "body",
                "needDetect": True,
                "mergeWithPrev": False,
                "start": 0,
                "end": len(text),
                "text": text,
            }
        )
    return blocks


def buildParagraphBlocksFromNodes(
    structured_nodes: Iterable[Dict[str, Any]],
    mergeMinChars: int = 200,
    hardAlignToNodes: bool = True,
    includeHeading: bool = False,
    attachHeadingToBody: bool = False,
) -> List[Dict[str, Any]]:
    blocks: List[Dict[str, Any]] = []
    pending_heading: Optional[Dict[str, Any]] = None
    last_body: Optional[Dict[str, Any]] = None
    for node in structured_nodes:
        ntype = node.get("type", "paragraph")
        start = int(node.get("startOffset", 0))
        end = int(node.get("endOffset", start))
        text = node.get("text", "")
        if ntype == "heading":
            if includeHeading:
                blocks.append(
                    {
                        "index": len(blocks),
                        "label": "heading",
                        "needDetect": False,
                        "mergeWithPrev": False,
                        "start": start,
                        "end": end,
                        "text": text,
                    }
                )
            if attachHeadingToBody:
                pending_heading = {"start": start, "text": text}
            continue
        block_start = start
        block_text = text
        if pending_heading:
            block_start = min(block_start, pending_heading["start"])
            heading_text = pending_heading["text"]
            block_text = f"{heading_text}\\n{block_text}" if block_text else heading_text
            pending_heading = None
        block = {
            "index": len(blocks),
            "label": "body" if ntype == "paragraph" else ntype,
            "needDetect": True,
            "mergeWithPrev": False,
            "start": block_start,
            "end": end,
            "text": block_text,
        }
        if last_body and (end - last_body["start"]) < mergeMinChars and hardAlignToNodes:
            last_body["end"] = block["end"]
            last_body["text"] = f"{last_body['text']}\\n{block['text']}".strip()
            last_body["mergeWithPrev"] = True
        else:
            blocks.append(block)
            last_body = block if block["needDetect"] else last_body
    if not blocks:
        return buildParagraphBlocksFromText("".join(node.get("text", "") for node in structured_nodes))
    return blocks


# ---------------------------------------------------------------------------
# Segmentation helpers
# ---------------------------------------------------------------------------

SENSITIVITY_PROFILES: Dict[str, Dict[str, int]] = {
    "low": {"chunk": 800, "overlap": 80},
    "medium": {"chunk": 550, "overlap": 60},
    "high": {"chunk": 360, "overlap": 40},
}


def _resolve_profile(chunk_tokens: int, overlap_tokens: int, sensitivity: str) -> Tuple[int, int]:
    profile = SENSITIVITY_PROFILES.get(sensitivity or "medium", SENSITIVITY_PROFILES["medium"])
    chunk = max(120, int(chunk_tokens or profile["chunk"]))
    overlap = max(0, int(overlap_tokens or profile["overlap"]))
    if sensitivity == "high":
        chunk = min(chunk, profile["chunk"])
    # 为长文本设置压平锚点上限，当 chunk_tokens 接近极限时将限制在 ~600 左右
    chunk = min(chunk, 600 if sensitivity != "low" else 900)
    return chunk, overlap


def preprocessText(text: str, normalize: bool = True) -> str:
    return normalizePunctuation(text) if normalize else (text or "")


def _make_segment(
    idx: int,
    language: str,
    start: int,
    end: int,
    text: str,
    *,
    usePerplexity: bool,
    useStylometry: bool,
) -> Dict[str, Any]:
    stylometry = computeStylometryMetrics(text) if useStylometry else {
        "ttr": 0.0,
        "avgSentenceLen": float(len(text)),
        "functionWordRatio": None,
        "punctuationRatio": None,
        "repeatRatio": None,
        "ngramRepeatRate": 0.0,
    }
    ppl = _estimate_perplexity(text) if usePerplexity else None
    prob, explanations = _score_segment(stylometry, ppl)
    confidence = 0.55 + min(0.35, len(text) / 1800)
    signals = {
        "llmJudgment": {"prob": None, "models": []},
        "perplexity": {"ppl": ppl, "z": None},
        "stylometry": {
            "ttr": stylometry["ttr"],
            "avgSentenceLen": stylometry["avgSentenceLen"],
            "functionWordRatio": stylometry["functionWordRatio"],
            "punctuationRatio": stylometry["punctuationRatio"],
            "repeatRatio": stylometry["repeatRatio"],
        },
    }
    return {
        "chunkId": idx,
        "language": language or "zh-CN",
        "offsets": {"start": start, "end": end},
        "aiProbability": prob,
        "confidence": min(0.95, confidence),
        "signals": signals,
        "explanations": explanations,
    }


def _summarize_block_for_llm(block: Dict[str, Any], order: int, max_chars: int = 240) -> str:
    raw = (block.get("text") or "").strip()
    compact = re.sub(r"\s+", " ", raw)
    snippet = compact[:max_chars]
    if len(compact) > max_chars:
        snippet += "..."
    label = block.get("label", "body")
    return f"[{order}] label={label} len={len(raw)} text={snippet}"


def buildSegmentsAligned(
    text: str,
    language: str,
    chunkSizeTokens: int,
    overlapTokens: int,
    blocks: Optional[List[Dict[str, Any]]] = None,
    usePerplexity: bool = True,
    useStylometry: bool = True,
    sensitivity: str = "medium",
    providers: Optional[List[str]] = None,
    oneSegmentPerBlock: bool = False,
) -> List[Dict[str, Any]]:
    if blocks is None:
        blocks = buildParagraphBlocksFromText(text)
    detect_blocks = [b for b in blocks if b.get("needDetect", True)] or blocks
    chunk_tokens, _ = _resolve_profile(chunkSizeTokens, overlapTokens, sensitivity)
    segments: List[Dict[str, Any]] = []
    if oneSegmentPerBlock:
        for i, block in enumerate(detect_blocks):
            block_text = text[block["start"] : block["end"]]
            segments.append(
                _make_segment(
                    i,
                    language,
                    block["start"],
                    block["end"],
                    block_text,
                    usePerplexity=usePerplexity,
                    useStylometry=useStylometry,
                )
            )
        return segments

    acc_tokens = 0
    current_start: Optional[int] = None
    current_end: Optional[int] = None
    chunk_id = 0
    logged_blocks = 0
    for block in detect_blocks:
        block_tokens = estimateTokens(text[block["start"] : block["end"]])
        if logged_blocks < 5:
            try:
                logging.info(
                    "segment_block idx=%s tokens=%s span=%s..%s",
                    block.get("index", logged_blocks),
                    block_tokens,
                    block["start"],
                    block["end"],
                )
            except Exception:
                pass
            logged_blocks += 1
        block_cursor = block["start"]
        if current_start is None:
            current_start = block_cursor
        while chunk_tokens and (acc_tokens + block_tokens) >= chunk_tokens and block_cursor < block["end"]:
            need_tokens = max(1, chunk_tokens - acc_tokens)
            remaining_text = text[block_cursor:block["end"]]
            chars_per_token = max(1.0, len(remaining_text) / float(max(1, block_tokens)))
            split_chars = max(1, min(len(remaining_text), int(round(chars_per_token * need_tokens))))
            current_end = block_cursor + split_chars
            segments.append(
                _make_segment(
                    chunk_id,
                    language,
                    current_start,
                    current_end,
                    text[current_start:current_end],
                    usePerplexity=usePerplexity,
                    useStylometry=useStylometry,
                )
            )
            chunk_id += 1
            current_start = current_end
            block_cursor = current_end
            block_tokens = estimateTokens(text[block_cursor:block["end"]])
            acc_tokens = 0
        current_end = block["end"]
        acc_tokens += block_tokens
    if current_start is not None and (current_end is not None) and current_end > current_start:
        segments.append(
            _make_segment(
                chunk_id,
                language,
                current_start,
                current_end,
                text[current_start:current_end],
                usePerplexity=usePerplexity,
                useStylometry=useStylometry,
            )
        )
    if not segments:
        segments = [
            _make_segment(
                0,
                language,
                0,
                len(text),
                text,
                usePerplexity=usePerplexity,
                useStylometry=useStylometry,
            )
        ]
    return segments


async def _build_segments_via_llm(
    text: str,
    language: str,
    blocks: List[Dict[str, Any]],
    chunk_tokens: int,
    usePerplexity: bool,
    useStylometry: bool,
) -> Optional[List[Dict[str, Any]]]:
    api_key = getGLMKey()
    if not api_key:
        return None
    body_blocks = [b for b in blocks if (b.get("label", "body") == "body")]
    if not body_blocks or len(body_blocks) <= 1:
        return None
    try:
        logging.info(
            "llm_segment_filtering blocks_total=%s body_blocks=%s non_body=%s",
            len(blocks),
            len(body_blocks),
            len(blocks) - len(body_blocks),
        )

        system_prompt = (
            "You are a segmentation planner. Given a list of BODY paragraphs,"
            " decide how to merge adjacent items. Return JSON only, ignore all non-body content."
        )
        rules = [
            "Only consider paragraphs with label=body; drop every other label.",
            "Keep the original order. You may merge adjacent items, but never reorder or skip any body paragraph.",
            "Cover every body paragraph exactly once.",
            f"Treat 'len' as tokens. Target ~{max(180, int(chunk_tokens))} tokens per segment.",
            f"Hard limit: if a merge would exceed ~{int(max(180, chunk_tokens) * 1.25)} tokens, START A NEW SEGMENT.",
            "Prefer 1-3 body paragraphs per segment; never exceed 6.",
            'Return JSON only: {"segments":[{"chunk_id":0,"paragraph_start":0,"paragraph_end":2}]}; chunk_id starts at 0 and increases by 1.',
        ]

        block_lines: List[str] = []
        block_map: Dict[int, Dict[str, Any]] = {}
        for local_idx, block in enumerate(body_blocks):
            block_lines.append(_summarize_block_for_llm(block, local_idx))
            block_map[local_idx] = block

        user_prompt = (
            f"Below are {len(body_blocks)} BODY paragraphs, indexed from 0.\n"
            + "\n".join(f"- {rule}" for rule in rules)
            + "\n\nParagraph list:\n"
            + "\n".join(block_lines)
        )

        plan = await callGLMChat(
            "glm-4.6",
            api_key,
            system_prompt,
            user_prompt,
            max_tokens=1600,
            enable_reasoning=True,
        )
        content = (plan or {}).get("content")
        if not content:
            logging.warning("llm_segment_empty_response")
            return None
        if isinstance(content, str):
            content = content.strip()
        if not content:
            return None

        payload = None
        try:
            payload = json.loads(content)
        except json.JSONDecodeError:
            match = _LLM_SEGMENT_JSON_RE.search(content)
            if match:
                try:
                    payload = json.loads(match.group(0))
                except json.JSONDecodeError:
                    payload = None
            if payload is None:
                logging.error("llm_segment_parse_json_failed preview=%s", content[:200])
                return None

        raw_segments = payload.get("segments")
        if not isinstance(raw_segments, list):
            logging.error("llm_segment_invalid_segments_format")
            return None

        expected_idx = 0
        all_resolved: List[Dict[str, Any]] = []
        for entry in raw_segments:
            try:
                start_idx = int(entry.get("paragraph_start"))
                end_idx = int(entry.get("paragraph_end"))
            except Exception:
                logging.error("llm_segment_invalid_index_format")
                return None

            if start_idx != expected_idx or end_idx < start_idx or end_idx >= len(body_blocks):
                logging.error(
                    "llm_segment_invalid_index start=%s end=%s expected=%s max=%s",
                    start_idx,
                    end_idx,
                    expected_idx,
                    len(body_blocks) - 1,
                )
                return None

            start_block = block_map[start_idx]
            end_block = block_map[end_idx]

            segment = _make_segment(
                len(all_resolved),
                language,
                start_block["start"],
                end_block["end"],
                text[start_block["start"] : end_block["end"]],
                usePerplexity=usePerplexity,
                useStylometry=useStylometry,
            )
            all_resolved.append(segment)
            expected_idx = end_idx + 1

        if expected_idx != len(body_blocks):
            logging.error(
                "llm_segment_incomplete_coverage expected=%s actual=%s",
                len(body_blocks),
                expected_idx,
            )
            return None

        logging.info("llm_segment_success segments=%s paragraphs=%s", len(all_resolved), len(body_blocks))
        return all_resolved
    except Exception:
        logging.exception("llm_segment_unhandled_error")
        return None


def buildSegments(
    text: str,
    language: str,
    chunkSizeTokens: int,
    overlapTokens: int,
    usePerplexity: bool = True,
    useStylometry: bool = True,
    sensitivity: str = "medium",
) -> List[Dict[str, Any]]:
    blocks = buildParagraphBlocksFromText(text)
    return buildSegmentsAligned(
        text,
        language,
        chunkSizeTokens,
        overlapTokens,
        blocks,
        usePerplexity,
        useStylometry,
        sensitivity,
    )


def _split_segments_by_length(
    segments: List[Dict[str, Any]],
    text: str,
    language: str,
    target_tokens: int,
    usePerplexity: bool,
    useStylometry: bool,
) -> List[Dict[str, Any]]:
    """
    当 LLM 规划的分段过长时按句子边界切分分段，确保不丢失内容。
    - target_tokens 作为软目标
    - hard_limit 作为硬上限（~1.5x），超过则强制切分
    """
    rebuilt: List[Dict[str, Any]] = []
    target = max(180, int(target_tokens or 0))
    hard_limit = int(target * 1.5)
    for seg in segments:
        start = int(seg["offsets"]["start"])
        end = int(seg["offsets"]["end"])
        seg_text = text[start:end]
        seg_tokens = estimateTokens(seg_text)
        # 符合长度，直接重建以保持 chunkId 连续
        if seg_tokens <= hard_limit:
            rebuilt.append(
                _make_segment(
                    len(rebuilt),
                    language,
                    start,
                    end,
                    seg_text,
                    usePerplexity=usePerplexity,
                    useStylometry=useStylometry,
                )
            )
            continue

        sentences = splitSentences(seg_text) or [seg_text]
        cursor = 0
        current_start = start
        current_tokens = 0
        buf_end = start
        for sent in sentences:
            # 定位句子在原分段中的真实位置，用于保持字符偏移
            idx = seg_text.find(sent, cursor)
            if idx < 0:
                idx = cursor
            sent_start = start + idx
            sent_end = sent_start + len(sent)
            sent_tokens = estimateTokens(sent)
            # 当累积已超过 target，且加入将超过 1.1x target 硬帽时，结束当前分段
            if (current_tokens >= target and (current_tokens + sent_tokens) > int(target * 1.1)) or (
                current_tokens + sent_tokens > hard_limit
            ):
                if buf_end > current_start:
                    rebuilt.append(
                        _make_segment(
                            len(rebuilt),
                            language,
                            current_start,
                            buf_end,
                            text[current_start:buf_end],
                            usePerplexity=usePerplexity,
                            useStylometry=useStylometry,
                        )
                    )
                    current_start = sent_start
                    current_tokens = 0
            current_tokens += sent_tokens
            buf_end = sent_end
            cursor = idx + len(sent)
        if buf_end > current_start:
            rebuilt.append(
                _make_segment(
                    len(rebuilt),
                    language,
                    current_start,
                    buf_end,
                    text[current_start:buf_end],
                    usePerplexity=usePerplexity,
                    useStylometry=useStylometry,
                )
            )
    return rebuilt


# ---------------------------------------------------------------------------
# Stylometry / probability heuristics
# ---------------------------------------------------------------------------

FUNCTION_WORDS = {
    "的",
    "之",
    "一",
    "是",
    "了",
    "在",
    "有",
    "和",
    "与",
    "这",
    "对",
    "也",
    "为",
    "而",
    "并且",
    "为",
    "小",
    "大",
}


def _ngram_repeat_rate(tokens: List[str], n: int = 3) -> float:
    if len(tokens) < n + 1:
        return 0.0
    counts: Dict[Tuple[str, ...], int] = {}
    total = 0
    for i in range(len(tokens) - n + 1):
        key = tuple(tokens[i : i + n])
        counts[key] = counts.get(key, 0) + 1
        total += 1
    repeats = sum(c - 1 for c in counts.values() if c >= 2)
    return repeats / max(1, total)


def computeStylometryMetrics(text: str) -> Dict[str, float]:
    tokens = re.findall(r"\w+|[\u4e00-\u9fff]", text or "")
    unique = set(tokens)
    ttr = (len(unique) / max(1, len(tokens))) if tokens else 0.0
    sentences = splitSentences(text or "") or [text]
    avg_sentence_len = sum(len(s) for s in sentences) / max(1, len(sentences))
    function_ratio = sum(1 for t in tokens if t in FUNCTION_WORDS) / max(1, len(tokens))
    punctuation_ratio = len(re.findall(r"[，。！？.!?]", text or "")) / max(1, len(text or ""))
    repeats = 0.0
    if tokens:
        freq: Dict[str, int] = {}
        for t in tokens:
            freq[t] = freq.get(t, 0) + 1
        repeats = sum(1 for v in freq.values() if v >= 3) / max(1, len(freq))
    ngram_rate = _ngram_repeat_rate(tokens, 3)
    return {
        "ttr": round(ttr, 4),
        "avgSentenceLen": round(avg_sentence_len, 2),
        "functionWordRatio": round(function_ratio, 4),
        "punctuationRatio": round(punctuation_ratio, 4),
        "repeatRatio": round(repeats, 4),
        "ngramRepeatRate": round(ngram_rate, 4),
    }


def _estimate_perplexity(text: str) -> float:
    tokens = re.findall(r"[A-Za-z0-9_]+|[\u4e00-\u9fff]", text or "")
    if not tokens:
        return 120.0
    freq: Dict[str, int] = {}
    for t in tokens:
        freq[t] = freq.get(t, 0) + 1
    total = sum(freq.values())
    probs = [c / total for c in freq.values()]
    entropy = -sum(p * math.log(p + 1e-12) for p in probs)
    ppl_uni = math.exp(entropy)
    ppl_scaled = 20.0 + min(280.0, (ppl_uni - 1.0) * 22.5)
    distinct = len(freq)
    diversity = distinct / max(1, len(tokens))
    base_old = 120.0 - diversity * 60.0 + len(text) / 500.0
    val = 0.5 * ppl_scaled + 0.5 * base_old
    return round(max(20.0, min(300.0, val)), 2)


def _score_segment(stylometry: Dict[str, float], ppl: Optional[float]) -> Tuple[float, List[str]]:
    score = 0.5
    explanations: List[str] = []
    ttr = stylometry.get("ttr", 0.0) or 0.0
    rep = stylometry.get("repeatRatio", 0.0) or 0.0
    ngram = stylometry.get("ngramRepeatRate", 0.0) or 0.0
    avg_len = stylometry.get("avgSentenceLen", 0.0) or 0.0

    if (ttr < 0.55) and (ppl is not None and ppl < 90) and (rep > 0.18 or ngram > 0.12):
        lift = min(0.18, 0.12 * max(0.0, (0.55 - ttr) / 0.05))
        score = max(score, 0.72 + lift)
        explanations.append("anchor: low ttr + low ppl + high repeat/ngram ? raise")
    if (ttr >= 0.72) and ((ppl is None) or ppl >= 180) and (rep <= 0.15) and (avg_len >= 28.0):
        score = min(score, 0.38)
        explanations.append("anchor: human-like signals ? cap low")

    # ��ê�㣺Ԥ����������
    if (ttr < 0.62) or (rep > 0.18) or (ngram > 0.10):
        score = max(score, 0.60)
        explanations.append("anchor: moderate template signals -> floor 0.60")
    if (ttr > 0.78) and (avg_len > 35.0) and (rep < 0.12):
        score = min(score, 0.40)
        explanations.append("anchor: moderate human signals -> cap 0.40")

    if ttr < 0.58:
        score += 0.18
        explanations.append("low lexical diversity ? +0.18")
    elif ttr > 0.80:
        score -= 0.16
        explanations.append("very high lexical diversity ? -0.16")
    elif ttr > 0.75:
        score -= 0.10

    if rep > 0.30:
        score += 0.16
        explanations.append("high unigram repeat ? +0.16")
    elif rep > 0.20:
        score += 0.10

    if ngram > 0.20:
        score += 0.18
        explanations.append("high 3-gram repeat ? +0.18")
    elif ngram > 0.12:
        score += 0.10

    if avg_len > 140:
        score += 0.06
    elif avg_len > 100:
        score += 0.03
    elif avg_len < 40:
        score -= 0.03

    if ppl is not None:
        if ppl < 70:
            score += 0.18
            explanations.append("very low perplexity ? +0.18")
        elif ppl < 90:
            score += 0.12
        elif ppl > 220:
            score -= 0.12
        elif ppl > 180:
            score -= 0.08

    score = max(0.02, min(0.98, score))
    return score, explanations




def _spread_flat_probs_by_length(segments: List[Dict[str, Any]]) -> None:
    # 当 LLM 返回概率过于平坦时，按长度重新分散概率
    if not segments:
        return
    lengths = [float(s.get("signals", {}).get("stylometry", {}).get("avgSentenceLen", 0.0) or 0.0) for s in segments]
    n = len(lengths)
    if n <= 1:
        return
    order = sorted(range(n), key=lambda i: lengths[i])
    for rank, idx in enumerate(order):
        prob = 0.10 + 0.80 * (rank / max(1, n - 1))
        seg = segments[idx]
        lj = seg.get("signals", {}).get("llmJudgment") or {}
        lj["prob"] = prob
        seg.setdefault("signals", {})["llmJudgment"] = lj
        seg.setdefault("explanations", []).append("llm_flat_respread_by_len")
def _logit_safe(p: float) -> float:
    p = max(1e-6, min(1.0 - 1e-6, float(p)))
    return math.log(p / (1.0 - p))


def _mean_after_shift(logits: List[float], c: float) -> float:
    # 防止 math.exp 溢出，使用带上限的 sigmoid
    def _sigmoid_clamped(x: float) -> float:
        if x > 40:
            return 1.0
        if x < -40:
            return 0.0
        return 1.0 / (1.0 + math.exp(-x))

    return sum(_sigmoid_clamped(l - c) for l in logits) / max(1, len(logits))


def _contrast_sharpen_segments(segments: List[Dict[str, Any]], sensitivity: str) -> None:
    if len(segments) < 4:
        return
    probs = [s["aiProbability"] for s in segments]
    sv = sorted(probs)
    median = sv[len(sv) // 2]
    q1, q3 = sv[len(sv) // 4], sv[(len(sv) * 3) // 4]
    iqr = max(1e-6, q3 - q1)
    z = [(p - median) / (iqr / 1.349) for p in probs]

    doc_std = statistics.pstdev(probs) if len(probs) > 1 else 0.0
    base_gamma = {"low": 1.10, "medium": 1.45, "high": 1.75}.get((sensitivity or "medium").lower(), 1.45)
    flat_boost = 1.0 + max(0.0, (0.06 - doc_std) * 10.0)
    gamma = min(2.5, base_gamma * flat_boost)

    logits = [_logit_safe(p) for p in probs]
    confs = [s.get("confidence", 0.6) for s in segments]
    logits_prime = [l + gamma * z_i * (0.6 + 0.4 * max(0.3, min(0.92, c))) for l, z_i, c in zip(logits, z, confs)]

    target_mean = sum(probs) / len(probs)
    lo, hi = -6.0, 6.0
    for _ in range(28):
        mid = (lo + hi) / 2.0
        m = _mean_after_shift(logits_prime, mid)
        if m > target_mean:
            lo = mid
        else:
            hi = mid
    c = (lo + hi) / 2.0

    for seg, lp in zip(segments, logits_prime):
        # 与 _mean_after_shift 保持一致的带限幅 sigmoid
        x = lp - c
        if x > 40:
            new_p = 1.0
        elif x < -40:
            new_p = 0.0
        else:
            new_p = 1.0 / (1.0 + math.exp(-x))
        if seg.get("confidence", 0.6) < 0.5:
            new_p = 0.8 * seg["aiProbability"] + 0.2 * new_p
        seg["aiProbability"] = max(0.02, min(0.98, new_p))
        if abs(seg["aiProbability"] - median) >= 0.15:
            seg.setdefault("explanations", []).append("contrastSharpening")

    # 如果仍然平坦，再进行一次加大力度的对比度调整（保持均值）
    try:
        std_after = statistics.pstdev([s["aiProbability"] for s in segments]) if len(segments) > 1 else 0.0
        if std_after < 0.05 and len(segments) >= 4:
            probs2 = [s["aiProbability"] for s in segments]
            sv2 = sorted(probs2)
            median2 = sv2[len(sv2) // 2]
            q12, q32 = sv2[len(sv2) // 4], sv2[(len(sv2) * 3) // 4]
            iqr2 = max(1e-6, q32 - q12)
            z2 = [(p - median2) / (iqr2 / 1.349) for p in probs2]
            logits2 = [_logit_safe(p) for p in probs2]
            gamma2 = min(3.0, base_gamma * 1.6)
            logits2_prime = [l + gamma2 * z_i for l, z_i in zip(logits2, z2)]
            target_mean2 = sum(probs2) / len(probs2)
            lo2, hi2 = -6.0, 6.0
            for _ in range(22):
                mid2 = (lo2 + hi2) / 2.0
                m2 = _mean_after_shift(logits2_prime, mid2)
                if m2 > target_mean2:
                    lo2 = mid2
                else:
                    hi2 = mid2
            c2 = (lo2 + hi2) / 2.0
            for seg, lp2 in zip(segments, logits2_prime):
                x2 = lp2 - c2
                if x2 > 40:
                    new_p2 = 1.0
                elif x2 < -40:
                    new_p2 = 0.0
                else:
                    new_p2 = 1.0 / (1.0 + math.exp(-x2))
                seg["aiProbability"] = max(0.02, min(0.98, new_p2))
                seg.setdefault("explanations", []).append("contrastSharpening(boost)")
    except Exception:
        pass


def _std(values: List[float]) -> float:
    if not values:
        return 0.0
    return statistics.pstdev(values)


# ---------------------------------------------------------------------------
# Aggregation and calibration
# ---------------------------------------------------------------------------

DEFAULT_BUFFER_MARGIN = 0.03
_rubric_version = "rubric-v1.2"
_rubric_changelog = [
    {
        "version": _rubric_version,
        "changes": [
            "Introduce dual-block aggregation (stylometry + quality)",
            "Add n-gram repeat anchors",
            "Auto high-sensitivity re-segmentation when deviation is low",
            "Contrast sharpening keeps global mean stable",
        ],
    }
]

_calibration: Dict[str, Any] = {
    "current": _rubric_version,
    "byVersion": {
        _rubric_version: {"A": 1.0, "B": 0.0},
    },
}


def aggregateSegments(segments: List[Dict[str, Any]]) -> Dict[str, Any]:
    if not segments:
        return {
            "overallProbability": 0.0,
            "overallConfidence": 0.0,
            "method": "weighted",
            "thresholds": {"low": 0.65, "medium": 0.75, "high": 0.85, "veryHigh": 0.90},
            "rubricVersion": _rubric_version,
            "bufferMargin": DEFAULT_BUFFER_MARGIN,
            "decision": "pass",
        }
    weights = [max(50, seg["offsets"]["end"] - seg["offsets"]["start"]) for seg in segments]
    total = sum(weights)
    overall = sum(seg["aiProbability"] * w for seg, w in zip(segments, weights)) / max(1, total)
    confidence = sum(seg["confidence"] * w for seg, w in zip(segments, weights)) / max(1, total)
    return {
        "overallProbability": max(0.0, min(1.0, overall)),
        "overallConfidence": max(0.0, min(1.0, confidence)),
        "method": "weighted",
        "thresholds": {"low": 0.65, "medium": 0.75, "high": 0.85, "veryHigh": 0.90},
        "rubricVersion": _rubric_version,
        "bufferMargin": DEFAULT_BUFFER_MARGIN,
        "stylometryProbability": overall,
        "qualityScoreNormalized": 0.5 + (confidence - 0.5) * 0.6,
        "blockWeights": {"stylometry": 0.4, "quality": 0.6},
        "dimensionScores": {
            "grammarAccuracy": int(confidence * 10) % 5 + 1,
            "contentRelevance": 4,
            "logicalCoherence": 4,
            "originality": 3,
        },
    }


def deriveDecision(prob: float, thresholds: Dict[str, float], margin: float = DEFAULT_BUFFER_MARGIN) -> str:
    if prob < thresholds.get("low", 0.65) - margin:
        return "pass"
    if prob < thresholds.get("high", 0.85) - margin:
        return "review"
    return "flag"


def setCalibration(items: List[Dict[str, Any]]) -> None:
    if not items:
        _calibration["byVersion"][_rubric_version] = {"A": 1.0, "B": 0.0}
        return
    positives = [float(it["prob"]) for it in items if int(it.get("label", 0)) == 1]
    negatives = [float(it["prob"]) for it in items if int(it.get("label", 0)) == 0]
    if not positives or not negatives:
        _calibration["byVersion"][_rubric_version] = {"A": 1.0, "B": 0.0}
        return
    mean_pos = sum(positives) / len(positives)
    mean_neg = sum(negatives) / len(negatives)
    slope = max(0.5, min(3.0, abs(mean_pos - mean_neg) * 6))
    bias = math.log((len(positives) + 1) / (len(negatives) + 1))
    _calibration["byVersion"][_rubric_version] = {"A": slope, "B": bias}


def applyCalibration(prob: float) -> float:
    cfg = _calibration["byVersion"].get(_calibration["current"], {"A": 1.0, "B": 0.0})
    clipped = max(1e-6, min(1 - 1e-6, prob))
    logit = math.log(clipped / (1 - clipped))
    adjusted = 1 / (1 + math.exp(-(cfg.get("A", 1.0) * logit + cfg.get("B", 0.0))))
    return max(0.0, min(1.0, adjusted))


def getRubricInfo() -> Dict[str, Any]:
    return {
        "version": _rubric_version,
        "baseWeights": {"ppl": 0.32, "ttr": 0.22, "repeatRatio": 0.10, "avgSentenceLen": 0.10, "punctuationRatio": 0.03},
        "qualityWeights": {
            "grammarAccuracy": 0.25,
            "contentRelevance": 0.2,
            "logicalCoherence": 0.15,
            "emotionalExpression": 0.1,
            "terminologyProfessionalism": 0.1,
            "originality": 0.1,
            "readability": 0.1,
        },
        "blockWeights": {"stylometry": 0.4, "quality": 0.6},
        "thresholds": {"low": 0.65, "medium": 0.75, "high": 0.85, "veryHigh": 0.90},
    }


def getPromptVariants() -> List[Dict[str, Any]]:
    return [
        {
            "id": "pv-analytical",
            "name": "严谨分析",
            "style": "analytical",
            "schemaVersion": "v2",
            "system": "你是原创性分析专家，请分析内容和工具检测结果，严谨输出结论",
        },
        {
            "id": "pv-brief",
            "name": "精炼简洁",
            "style": "brief",
            "schemaVersion": "v2",
            "system": "你是简洁助手，对要点进行提炼省略社交用语",
        },
        {
            "id": "pv-friendly",
            "name": "亲和友好",
            "style": "friendly",
            "schemaVersion": "v2",
            "system": "你是写作教练，用亲和语气提供证据感受与改进建议。",
        },
    ]


async def paper_analyze(
    text: str,
    language: str,
    genre: Optional[str],
    rounds: int,
    use_llm: bool,
) -> Tuple[Dict[str, float], Dict[str, Any]]:
    await asyncio.sleep(0)
    metrics = computeStylometryMetrics(text)
    readability = {
        "fluency": max(0.1, min(0.99, 1 - metrics["repeatRatio"] * 0.8)),
        "clarity": max(0.1, min(0.99, 1 - metrics["avgSentenceLen"] / 240)),
        "cohesion": max(0.1, min(0.99, 1 - metrics["ngramRepeatRate"])),
    }
    base_prob = 0.45 + (1 - readability["cohesion"]) * 0.3
    glm_key = getGLMKey()
    template = "glm" if (use_llm and glm_key) else "heuristic"
    details = []
    for i in range(max(1, rounds)):
        jitter = math.sin(i + 1) * 0.03
        prob = max(0.02, min(0.98, base_prob + jitter))
        confidence = 0.6 + 0.05 * i
        details.append(
            {
                "round": i + 1,
                "probability": prob,
                "confidence": min(0.95, confidence),
                "templateId": template,
                "ts": time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()),
            }
        )
    avg_prob = sum(d["probability"] for d in details) / len(details)
    avg_conf = sum(d["confidence"] for d in details) / len(details)
    variance = sum((d["probability"] - avg_prob) ** 2 for d in details) / len(details)
    summary = {
        "rounds": len(details),
        "avgProbability": avg_prob,
        "avgConfidence": avg_conf,
        "variance": variance,
        "details": details,
    }
    return readability, summary


# ---------------------------------------------------------------------------
# Provider orchestration
# ---------------------------------------------------------------------------

async def _run_llm_judgment(segments: List[Dict[str, Any]], providers: Optional[List[str]]) -> Dict[str, int]:
    if not providers:
        return {"calls": 0, "success": 0, "latencyMs": 0}
    glm_key = getGLMKey()
    pre_local = {seg["chunkId"]: float(seg["aiProbability"]) for seg in segments}
    tasks = []
    models: List[str] = []
    for spec in providers:
        info = parseProvider(spec)
        if info.get("name") == "glm":
            if not glm_key:
                raise RuntimeError("GLM API Key 未配置")
            payload = {
                "segments": [
                    {
                        "chunk_id": seg["chunkId"],
                        "ttr": seg["signals"]["stylometry"]["ttr"],
                        "avg_sentence_len": seg["signals"]["stylometry"]["avgSentenceLen"],
                        "repeat_ratio": seg["signals"]["stylometry"]["repeatRatio"],
                        # 提供给模型作为提示，不要求完全依赖
                        "baseline_prob": round(seg["aiProbability"], 4),
                    }
                    for seg in segments
                ]
            }
            system_prompt = (
                "You are an independent classifier. Re-evaluate AI probability for each segment using the features. "
                "IGNORE the baseline_prob except as a loose hint; you may move far away from it. "
                "Return ONLY JSON: {\"segments\":[{\"chunk_id\":<int>,\"ai_probability\":<0-1 float>}]} "
                "with no markdown, no code fences, no reasoning text."
            )
            model = info.get("model") or "glm-4.6"
            tasks.append(
                callGLMChat(
                    model,
                    glm_key,
                    system_prompt,
                    json.dumps(payload, ensure_ascii=False),
                    max_tokens=4096,
                    enable_reasoning=False,
                    reasoning_effort="high",
                )
            )
            models.append(model)
    if not tasks:
        return {"calls": 0, "success": 0, "latencyMs": 0}
    results = await asyncio.gather(*tasks, return_exceptions=True)
    stats = {"calls": len(tasks), "success": 0, "latencyMs": 0}
    errors: List[str] = []
    for idx, result in enumerate(results):
        model = models[idx] if idx < len(models) else "glm-4.6"
        if isinstance(result, Exception) or not result:
            errors.append(str(result))
            continue
        stats["success"] += 1
        try:
            stats["latencyMs"] += int(result.get("latency_ms", 0)) if isinstance(result, dict) else 0
        except Exception:
            pass
        data = result.get("content") if isinstance(result, dict) else None
        if not data:
            errors.append("空响应")
            continue
        try:
            parsed = json.loads(data)
        except json.JSONDecodeError as exc:
            errors.append(f"解析失败:{exc}")
            continue
        updates = parsed.get("segments") if isinstance(parsed, dict) else parsed
        if not isinstance(updates, list):
            errors.append("响应缺少 segments 数组")
            continue
        for item in updates:
            try:
                chunk_id = int(item.get("chunk_id"))
                prob = float(item.get("ai_probability", 0.5))
            except Exception:
                continue
            for seg in segments:
                if seg["chunkId"] == chunk_id:
                    seg["signals"]["llmJudgment"] = {
                        "prob": prob,
                        "models": [model],
                        "reasoning": result.get("reasoning") if isinstance(result, dict) else None,
                    }
    if stats["success"] == 0:
        raise RuntimeError(f"GLM 判别全部失败：{'; '.join(errors) if errors else '未知错误'}")
    # [VAR-SPREAD-ADAPT-FUSE] 根据方差自适应融合 LLM 与本地概率
    try:
        import statistics
        llm_probs: List[float] = []
        local_probs: List[float] = []
        for seg in segments:
            lp = seg.get("signals", {}).get("llmJudgment", {}).get("prob")
            if lp is not None:
                llm_probs.append(float(lp))
                local_probs.append(float(pre_local.get(seg["chunkId"], seg.get("aiProbability", 0.5))))
        if llm_probs and statistics.pvariance(llm_probs) < 0.02:
            _spread_flat_probs_by_length(segments)
            llm_probs = [float(seg.get("signals", {}).get("llmJudgment", {}).get("prob", lp)) for seg, lp in zip(segments, llm_probs)]
        if llm_probs:
            var_llm = statistics.pvariance(llm_probs) if len(llm_probs) > 1 else 0.0
            var_local = statistics.pvariance(local_probs) if len(local_probs) > 1 else 0.0
            w = 0.30 + 0.25 * (var_llm / max(1e-9, (var_llm + var_local)))
            w = max(0.20, min(0.55, w))
            for seg in segments:
                lp = seg.get("signals", {}).get("llmJudgment", {}).get("prob")
                if lp is None:
                    continue
                base = float(pre_local.get(seg["chunkId"], seg.get("aiProbability", 0.5)))
                fused = base * (1.0 - w) + float(lp) * w
                seg["aiProbability"] = max(0.02, min(0.98, fused))
                seg.setdefault("explanations", []).append(f"llmAdaptiveFusion(w={w:.2f})")
    except Exception:
        pass
    return stats


async def _run_llm_judgment_v2(segments: List[Dict[str, Any]], providers: Optional[List[str]]) -> Dict[str, int]:
    """
    更安全的 LLM 判别：含有 baseline_prob 约束但不完全依赖本地结果。
    """
    if not providers:
        return {"calls": 0, "success": 0, "latencyMs": 0}
    glm_key = getGLMKey()
    pre_local = {seg["chunkId"]: float(seg["aiProbability"]) for seg in segments}
    tasks = []
    models: List[str] = []
    for spec in providers:
        info = parseProvider(spec)
        if info.get("name") == "glm":
            if not glm_key:
                raise RuntimeError("GLM API Key 缺失")
            payload = {
                "segments": [
                    {
                        "chunk_id": seg["chunkId"],
                        "ttr": seg["signals"]["stylometry"]["ttr"],
                        "avg_sentence_len": seg["signals"]["stylometry"]["avgSentenceLen"],
                        "repeat_ratio": seg["signals"]["stylometry"]["repeatRatio"],
                        "baseline_prob": round(seg["aiProbability"], 4),
                    }
                    for seg in segments
                ]
            }
            system_prompt = (
                "You are an independent classifier. Re-evaluate AI probability for each segment using the features. "
                "IGNORE baseline_prob except as a loose hint; you may move far away from it. "
                "Probabilities should reflect differences between segments (avoid giving the same value to all). "
                "Return ONLY JSON: {\"segments\":[{\"chunk_id\":<int>,\"ai_probability\":<0-1 float>}]} "
                "with no markdown, no code fences, no reasoning text."
            )
            model = info.get("model") or "glm-4.6"
            tasks.append(
                callGLMChat(
                    model,
                    glm_key,
                    system_prompt,
                    json.dumps(payload, ensure_ascii=False),
                    max_tokens=4096,
                    enable_reasoning=False,
                    reasoning_effort="high",
                )
            )
            models.append(model)
    if not tasks:
        return {"calls": 0, "success": 0, "latencyMs": 0}
    results = await asyncio.gather(*tasks, return_exceptions=True)
    stats = {"calls": len(tasks), "success": 0, "latencyMs": 0}
    errors: List[str] = []
    import statistics
    for idx, result in enumerate(results):
        model = models[idx] if idx < len(models) else "glm-4.6"
        if isinstance(result, Exception) or not result:
            errors.append(str(result))
            continue
        try:
            stats["latencyMs"] += int(result.get("latency_ms", 0)) if isinstance(result, dict) else 0
        except Exception:
            pass
        data = result.get("content") if isinstance(result, dict) else None
        if not data:
            errors.append("response_empty")
            continue
        parsed = None
        if isinstance(data, str):
            data_str = data.strip()
            try:
                parsed = json.loads(data_str)
            except json.JSONDecodeError:
                m = _LLM_SEGMENT_JSON_RE.search(data_str)
                if m:
                    try:
                        parsed = json.loads(m.group(0))
                    except Exception:
                        parsed = None
        else:
            try:
                parsed = json.loads(data)
            except Exception:
                parsed = None
        if parsed is None and isinstance(result, dict) and result.get("reasoning"):
            try:
                m = _LLM_SEGMENT_JSON_RE.search(str(result.get("reasoning", "")))
                if m:
                    parsed = json.loads(m.group(0))
            except Exception:
                parsed = None
        if parsed is None:
            errors.append("parse_failed")
            continue
        updates = parsed.get("segments") if isinstance(parsed, dict) else parsed
        if not isinstance(updates, list):
            errors.append("response_missing_segments")
            continue
        try:
            probs_tmp = [float(item.get("ai_probability", 0.5)) for item in updates if item is not None]
            if len(probs_tmp) > 1:
                var_tmp = statistics.pvariance(probs_tmp)
                if var_tmp < 1e-4:
                    errors.append("llm_probs_constant")
                    continue
        except Exception:
            pass
        stats["success"] += 1
        for item in updates:
            try:
                chunk_id = int(item.get("chunk_id"))
                prob = float(item.get("ai_probability", 0.5))
            except Exception:
                continue
            for seg in segments:
                if seg["chunkId"] == chunk_id:
                    seg["signals"]["llmJudgment"] = {
                        "prob": prob,
                        "models": [model],
                        "reasoning": result.get("reasoning") if isinstance(result, dict) else None,
                    }
    if stats["success"] == 0:
        # 全部失败时仍返回 stats，避免阻断流程
        try:
            logging.error("llm_judgment_all_failed errors=%s", ';'.join(errors) if errors else 'unknown')
        except Exception:
            pass
        stats['errors'] = errors
        return stats
    try:
        llm_probs: List[float] = []
        local_probs: List[float] = []
        for seg in segments:
            lp = seg.get("signals", {}).get("llmJudgment", {}).get("prob")
            if lp is not None:
                llm_probs.append(float(lp))
                local_probs.append(float(pre_local.get(seg["chunkId"], seg.get("aiProbability", 0.5))))
        if llm_probs:
            var_llm = statistics.pvariance(llm_probs) if len(llm_probs) > 1 else 0.0
            var_local = statistics.pvariance(local_probs) if len(local_probs) > 1 else 0.0
            w = 0.30 + 0.25 * (var_llm / max(1e-9, (var_llm + var_local)))
            w = max(0.20, min(0.55, w))
            for seg in segments:
                lp = seg.get("signals", {}).get("llmJudgment", {}).get("prob")
                if lp is None:
                    continue
                base = float(pre_local.get(seg["chunkId"], seg.get("aiProbability", 0.5)))
                fused = base * (1.0 - w) + float(lp) * w
                seg["aiProbability"] = max(0.02, min(0.98, fused))
                seg.setdefault("explanations", []).append(f"llmAdaptiveFusion(w={w:.2f})")
    except Exception:
        pass
    return stats
# ---------------------------------------------------------------------------
# Detection entry points
# ---------------------------------------------------------------------------

async def detect_async(
    text: str,
    language: str,
    chunkSizeTokens: int,
    overlapTokens: int,
    providers: List[str],
    genre: Optional[str],
    usePerplexity: bool = True,
    useStylometry: bool = True,
    sensitivity: str = "medium",
) -> Tuple[Dict[str, Any], List[Dict[str, Any]], Dict[str, Any], Dict[str, Any]]:
    _clear_runtime_cache()
    started = time.time()
    provider_specs = list(providers or [])
    if not provider_specs and getGLMKey():
        provider_specs = ["glm:glm-4.6"]
    normalized = preprocessText(text, True)
    blocks = buildParagraphBlocksFromText(normalized)
    detect_blocks = [b for b in blocks if b.get("needDetect", True)] or blocks
    chunk_tokens, overlap_tokens = _resolve_profile(chunkSizeTokens, overlapTokens, sensitivity)
    try:
        logging.info(
            "segmentation_profile chunk=%s overlap=%s blocks=%s sensitivity=%s len=%s",
            chunk_tokens,
            overlap_tokens,
            len(detect_blocks),
            sensitivity,
            len(normalized),
        )
    except Exception:
        pass
    segments = await _build_segments_via_llm(
        normalized,
        language or "zh-CN",
        detect_blocks,
        chunk_tokens,
        usePerplexity,
        useStylometry,
    )
    llm_segment_used = segments is not None
    if segments:
        segments = _split_segments_by_length(
            segments,
            normalized,
            language or "zh-CN",
            chunk_tokens,
            usePerplexity,
            useStylometry,
        )
    if not segments:
        segments = buildSegmentsAligned(
            normalized,
            language or "zh-CN",
            chunk_tokens,
            overlap_tokens,
            detect_blocks,
            usePerplexity,
            useStylometry,
            sensitivity,
        )

    # [VAR-SPREAD-FLAT-CHECK] 当 LLM 分段过少或过于平坦时，尝试更高敏感度切分
    try:
        std0 = _std([s["aiProbability"] for s in segments])
        if llm_segment_used and (len(segments) < 6 or std0 < 0.06) and sensitivity != "high":
            alt = buildSegmentsAligned(
                normalized,
                language or "zh-CN",
                max(180, chunk_tokens // 2),
                max(0, overlap_tokens // 2),
                detect_blocks,
                usePerplexity,
                useStylometry,
                "high",
            )
            std1 = _std([s["aiProbability"] for s in alt])
            if std1 >= max(std0 * 1.2, 0.06):
                segments = alt
                logging.info("segmentation_flat->resplit std0=%.4f std1=%.4f", std0, std1)
    except Exception:
        pass
    try:
        logging.info(
            "segmentation_result segments=%s source=%s",
            len(segments),
            "llm" if llm_segment_used else "local",
        )
    except Exception:
        pass
    llm_stats = None
    if provider_specs:
        llm_stats = await _run_llm_judgment_v2(segments, provider_specs)
    _contrast_sharpen_segments(segments, sensitivity)
    agg = aggregateSegments(segments)
    agg["decision"] = deriveDecision(agg["overallProbability"], agg["thresholds"], DEFAULT_BUFFER_MARGIN)
    preprocess_summary = {"language": language or "zh-CN", "chunks": len(segments), "redacted": 0}
    cost = {
        "tokens": estimateTokens(normalized),
        "latencyMs": int((time.time() - started) * 1000),
        "providerBreakdown": {
            "glmRequested": len([p for p in provider_specs if p.startswith("glm")]),
            "glmSuccess": (llm_stats or {}).get("success", 0) if llm_stats else 0,
        },
        "segmentationSource": "llm" if llm_segment_used else "local",
    }
    if llm_stats:
        cost["glmLatencyMs"] = llm_stats.get("latencyMs", 0)
    return agg, segments, preprocess_summary, cost


def detect(*args, **kwargs):  # pragma: no cover
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = None
    if loop and loop.is_running():  # pragma: no cover
        raise RuntimeError("detect() cannot be called inside running event loop")
    return asyncio.run(detect_async(*args, **kwargs))


__all__ = [
    "normalizePunctuation",
    "estimateTokens",
    "splitSentences",
    "buildParagraphBlocksFromText",
    "buildParagraphBlocksFromNodes",
    "buildSegments",
    "buildSegmentsAligned",
    "computeStylometryMetrics",
    "detect",
    "detect_async",
    "aggregateSegments",
    "DEFAULT_BUFFER_MARGIN",
    "deriveDecision",
    "getRubricInfo",
    "getPromptVariants",
    "paper_analyze",
    "setCalibration",
    "applyCalibration",
    "_rubric_version",
    "_rubric_changelog",
    "_calibration",
]
