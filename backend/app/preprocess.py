"""Shared preprocessing helpers for document uploads and tooling."""
from __future__ import annotations

import io
import re
from typing import Any, Dict, Iterable, List, Optional, Tuple

from fastapi import UploadFile

from .service import (
    buildParagraphBlocksFromNodes,
    buildSegmentsAligned,
    preprocessText,
)


HEADING_META_KEYWORDS = ["副标题", "Subheading", "Subtitle", "编号", "No.", "NO.", "序号"]


def decode_uploaded_file(name: str, data: bytes) -> str:
    lower = (name or "").lower()
    if lower.endswith(".txt"):
        for enc in ("utf-8", "gbk", "latin1"):
            try:
                return data.decode(enc)
            except Exception:
                continue
        return data.decode("utf-8", errors="ignore")
    if lower.endswith(".docx"):
        from docx import Document

        doc = Document(io.BytesIO(data))
        return "\n".join(p.text for p in doc.paragraphs)
    if lower.endswith(".pdf"):
        from pypdf import PdfReader

        reader = PdfReader(io.BytesIO(data))
        return "\n".join(page.extract_text() or "" for page in reader.pages)
    return data.decode("utf-8", errors="ignore")


def _is_heading_line(line: str) -> bool:
    if not line:
        return False
    if line.startswith(("#", "##", "###")):
        return True
    if line.endswith((":", "：")):
        return True
    if re.match(r"^第[0-9一二三四五六七八九十百千]+[章节部分]", line):
        return True
    if re.match(r"^(?:Chapter|CHAPTER)\s+\d+\b", line):
        return True
    if re.match(r"^(?:[IVXLCM]+\.|\d+(?:\.\d+)*[.,、])\s*\S+", line):
        return True
    if len(line) <= 30 and not line.endswith(("。", "！", "？", ".", "!", "?")):
        return True
    return False


def _is_list_line(line: str) -> bool:
    if line.startswith(("-", "•", "*")):
        return True
    if re.match(r"^\d+[\.)]\s+\S", line) and (
        len(line) > 30 or line.endswith(("。", "！", "？", ".", "!", "?"))
    ):
        return True
    return False


def _is_heading_meta_line(line: str) -> bool:
    if not line or len(line) > 40:
        return False
    if line.endswith((": ", "：")):
        return True
    if any(k in line for k in HEADING_META_KEYWORDS):
        return True
    if re.match(r"^\d{4}[-/年]\d{1,2}([-/月]\d{1,2})?", line):
        return True
    if re.match(r"^[A-Z]{2,}$", line):
        return True
    return False


def _heading_level(line: str) -> Optional[int]:
    if line.startswith("###"):
        return 3
    if line.startswith("##"):
        return 2
    if line.startswith("#"):
        return 1
    if re.match(r"^第[一二三四五六七八九十百千]+章", line):
        return 1
    if re.match(r"^第[一二三四五六七八九十百千]+节", line):
        return 2
    if re.match(r"^\d+\.\d+\.", line):
        return 3
    if re.match(r"^\d+\.\s*\S+", line):
        return 1
    return None


def build_structured_nodes(normalized: str) -> Tuple[List[Dict[str, Any]], str, Dict[str, int]]:
    lines = [ln.strip() for ln in normalized.split("\n")]
    line_starts: List[int] = []
    line_ends: List[int] = []
    cursor = 0
    for ln in lines:
        line_starts.append(cursor)
        line_ends.append(cursor + len(ln))
        cursor = line_ends[-1] + 1

    structured: List[Dict[str, Any]] = []
    i = 0
    while i < len(lines):
        ln = lines[i]
        start = line_starts[i]
        end = line_ends[i]
        if _is_heading_line(ln) and ln:
            meta_count = 0
            parts = [ln]
            j = i + 1
            while j < len(lines):
                nxt = lines[j].strip()
                if nxt and _is_heading_meta_line(nxt):
                    parts.append(nxt)
                    end = line_ends[j]
                    meta_count += 1
                    j += 1
                    continue
                break
            structured.append(
                {
                    "type": "heading",
                    "text": "\n".join(parts),
                    "startOffset": start,
                    "endOffset": end,
                    "sectionPath": f"H:{ln}",
                    "level": _heading_level(ln),
                    "metaCount": meta_count,
                }
            )
            i = j
            continue
        if _is_list_line(ln) and ln:
            structured.append(
                {
                    "type": "list_item",
                    "text": ln,
                    "startOffset": start,
                    "endOffset": end,
                    "sectionPath": "",
                }
            )
            i += 1
            continue
        if ln:
            structured.append(
                {
                    "type": "paragraph",
                    "text": ln,
                    "startOffset": start,
                    "endOffset": end,
                    "sectionPath": "",
                }
            )
        i += 1

    formatted_lines: List[str] = []
    for node in structured:
        if node["type"] == "heading":
            formatted_lines.append(f"## {node['text']}")
            formatted_lines.append("")
        elif node["type"] == "list_item":
            formatted_lines.append(f"• {node['text'].lstrip('-•').strip()}")
        else:
            formatted_lines.append(node["text"])
            formatted_lines.append("")

    format_summary = {
        "headings": sum(1 for n in structured if n["type"] == "heading"),
        "paragraphs": sum(1 for n in structured if n["type"] == "paragraph"),
        "listItems": sum(1 for n in structured if n["type"] == "list_item"),
    }

    return structured, "\n".join(formatted_lines).strip(), format_summary


def _build_segment_node_mapping(
    segments: Iterable[Dict[str, Any]], structured_nodes: List[Dict[str, Any]]
) -> Tuple[
    Dict[int, List[Dict[str, Any]]],
    Dict[int, List[Dict[str, Any]]],
    List[int],
    List[int],
    Dict[int, int],
]:
    seg_node_map: Dict[int, List[Dict[str, Any]]] = {}
    node_chunk_map: Dict[int, List[Dict[str, Any]]] = {}
    primary_node_for_chunk: Dict[int, int] = {}
    unmapped_chunks: List[int] = []
    unmapped_nodes: List[int] = []

    for seg in segments:
        sid = int(seg["chunkId"])
        s_start = seg["offsets"]["start"]
        s_end = seg["offsets"]["end"]
        hits: List[Dict[str, Any]] = []
        for idx, node in enumerate(structured_nodes):
            n_start = node["startOffset"]
            n_end = node["endOffset"]
            overlap = max(0, min(s_end, n_end) - max(s_start, n_start))
            if overlap <= 0:
                continue
            coverage = overlap / max(1, (n_end - n_start))
            if coverage < 0.2:
                continue
            hit = {
                "nodeIndex": idx,
                "overlapChars": overlap,
                "coverageRatio": round(coverage, 4),
            }
            hits.append(hit)
            node_chunk_map.setdefault(idx, []).append(
                {"chunkId": sid, "overlapChars": overlap, "coverageRatio": hit["coverageRatio"]}
            )
        if hits:
            hits.sort(key=lambda x: (-x["coverageRatio"], -x["overlapChars"]))
            body_hits = [
                h for h in hits if structured_nodes[h["nodeIndex"]]["type"] in {"paragraph", "list_item"}
            ]
            preferred = body_hits[0]["nodeIndex"] if body_hits else hits[0]["nodeIndex"]
            primary_node_for_chunk[sid] = preferred
            seg_node_map[sid] = hits
        else:
            unmapped_chunks.append(sid)

    for idx in range(len(structured_nodes)):
        if idx not in node_chunk_map:
            unmapped_nodes.append(idx)

    return seg_node_map, node_chunk_map, unmapped_chunks, unmapped_nodes, primary_node_for_chunk


def _find_mismatches(seg_node_map: Dict[int, List[Dict[str, Any]]]) -> List[Dict[str, Any]]:
    mismatches: List[Dict[str, Any]] = []
    for chunk_id, hits in seg_node_map.items():
        if not hits:
            continue
        if len(hits) >= 2 or hits[0].get("coverageRatio", 0) < 0.8:
            mismatches.append({"chunkId": chunk_id, "hits": hits})
    return mismatches


def _heading_body_assoc_ok(
    seg_node_map: Dict[int, List[Dict[str, Any]]],
    node_chunk_map: Dict[int, List[Dict[str, Any]]],
    structured_nodes: List[Dict[str, Any]],
) -> bool:
    heading_idx = [i for i, n in enumerate(structured_nodes) if n.get("type") == "heading"]
    for hi in heading_idx:
        chunks = node_chunk_map.get(hi, [])
        if not chunks:
            continue
        if any(
            structured_nodes[item.get("nodeIndex", -1)]["type"] in {"paragraph", "list_item"}
            for chunk in chunks
            for item in seg_node_map.get(chunk["chunkId"], [])
        ):
            continue
        return False
    return True


def preprocess_document(
    text: str,
    *,
    normalize_punctuation: bool,
    auto_language: bool,
    chunk_size_tokens: int,
    overlap_tokens: int,
) -> Dict[str, Any]:
    normalized = preprocessText(text, normalize_punctuation)
    structured, formatted_text, format_summary = build_structured_nodes(normalized)
    language = "zh-CN" if auto_language else ""

    before_blocks = buildParagraphBlocksFromNodes(
        structured, mergeMinChars=200, hardAlignToNodes=False, includeHeading=True
    )
    before_segments = buildSegmentsAligned(
        normalized,
        language or "zh-CN",
        chunk_size_tokens,
        overlap_tokens,
        before_blocks,
        True,
        True,
        "medium",
        None,
    )

    after_blocks = buildParagraphBlocksFromNodes(
        structured,
        mergeMinChars=100,
        hardAlignToNodes=True,
        includeHeading=True,
        attachHeadingToBody=True,
    )
    aligned_segments = buildSegmentsAligned(
        normalized,
        language or "zh-CN",
        chunk_size_tokens,
        overlap_tokens,
        after_blocks,
        True,
        True,
        "medium",
        None,
        oneSegmentPerBlock=True,
    )

    seg_node_map_before, node_chunk_map_before, unmapped_chunks_before, unmapped_nodes_before, _ = (
        _build_segment_node_mapping(before_segments, structured)
    )
    (
        seg_node_map_after,
        node_chunk_map_after,
        unmapped_chunks_after,
        unmapped_nodes_after,
        primary_after,
    ) = _build_segment_node_mapping(aligned_segments, structured)

    comparison = {
        "before": {
            "segmentCount": len(before_segments),
            "nodeCount": len(structured),
            "mismatches": _find_mismatches(seg_node_map_before),
            "unmappedChunks": unmapped_chunks_before,
            "unmappedNodes": unmapped_nodes_before,
            "headingBodyAssociationOk": _heading_body_assoc_ok(
                seg_node_map_before, node_chunk_map_before, structured
            ),
        },
        "after": {
            "segmentCount": len(aligned_segments),
            "nodeCount": len(structured),
            "mismatches": _find_mismatches(seg_node_map_after),
            "unmappedChunks": unmapped_chunks_after,
            "unmappedNodes": unmapped_nodes_after,
            "headingBodyAssociationOk": _heading_body_assoc_ok(
                seg_node_map_after, node_chunk_map_after, structured
            ),
        },
    }

    mapping = {
        "segmentNodeMap": seg_node_map_after,
        "nodeChunkMap": node_chunk_map_after,
        "unmappedChunks": unmapped_chunks_after,
        "unmappedNodes": unmapped_nodes_after,
        "primaryNodeForChunk": primary_after,
    }

    preprocess_summary = {"language": language or "zh-CN", "chunks": len(aligned_segments), "redacted": 0}

    return {
        "normalizedText": normalized,
        "structuredNodes": structured,
        "formattedText": formatted_text,
        "formatSummary": format_summary,
        "segments": aligned_segments,
        "preprocessSummary": preprocess_summary,
        "mapping": mapping,
        "comparison": comparison,
    }


async def preprocess_upload_file(
    file: UploadFile,
    *,
    normalize_punctuation: bool,
    auto_language: bool,
    chunk_size_tokens: int,
    overlap_tokens: int,
) -> Dict[str, Any]:
    data = await file.read()
    text = decode_uploaded_file(file.filename or "file", data)
    return preprocess_document(
        text,
        normalize_punctuation=normalize_punctuation,
        auto_language=auto_language,
        chunk_size_tokens=chunk_size_tokens,
        overlap_tokens=overlap_tokens,
    )
