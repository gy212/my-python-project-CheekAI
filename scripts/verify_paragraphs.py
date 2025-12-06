import json
from pathlib import Path
import sys

sys.path.append('.')

from backend.app.preprocess import build_structured_nodes, preprocess_document  # noqa: E402
from backend.app.service import buildParagraphBlocksFromNodes, buildSegmentsAligned  # noqa: E402


def summarize_sample(text: str) -> dict:
    result = preprocess_document(
        text,
        normalize_punctuation=True,
        auto_language=True,
        chunk_size_tokens=400,
        overlap_tokens=80,
    )
    return {
        'structuredCount': len(result['structuredNodes']),
        'formattedPreview': result['formattedText'][:80],
        'chunks': len(result['segments']),
        'mappingIntegrity': {
            'unmappedNodes': len(result['mapping']['unmappedNodes']),
            'unmappedChunks': len(result['mapping']['unmappedChunks']),
        },
    }


def verify_heading_merges() -> None:
    text = (
        '## 绪论\n副标题：研究背景\n\n'
        'Chapter 2: Methods\n'
        'Subheading: Overview\n\n'
        '1. 引言\n内容段落一\n\n'
        '# 标题\n## 子标题\n### 小节\n'
    )
    structured, _, _ = build_structured_nodes(text)
    headings = [n for n in structured if n['type'] == 'heading']
    assert len(headings) >= 4
    assert headings[0].get('metaCount', 0) >= 1
    assert any(h['text'].startswith('1. 引言') for h in headings)


def verify_alignment_consistency() -> None:
    sample = '# A\n正文一\n\n## B\n正文二\n'
    structured, _, _ = build_structured_nodes(sample)
    blocks = buildParagraphBlocksFromNodes(structured, 100, True, True, attachHeadingToBody=True)
    segments = buildSegmentsAligned(sample, 'zh-CN', 400, 80, blocks, True, True, oneSegmentPerBlock=True)
    assert len(segments) == 2
    first = segments[0]['offsets']
    second = segments[1]['offsets']
    assert first['start'] <= structured[0]['startOffset'] <= first['end']
    assert second['start'] <= structured[-1]['startOffset'] <= second['end']


def main() -> None:
    report = summarize_sample(Path('samples/test_paragraphs.txt').read_text(encoding='utf-8'))
    print(json.dumps(report, ensure_ascii=False))
    verify_heading_merges()
    verify_alignment_consistency()
    print(json.dumps({'headingIntegrityOk': True}, ensure_ascii=False))
    print(json.dumps({'alignmentOk': True}, ensure_ascii=False))


if __name__ == '__main__':
    main()
