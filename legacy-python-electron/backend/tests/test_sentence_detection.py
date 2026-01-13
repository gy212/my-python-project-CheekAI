# -*- coding: utf-8 -*-
"""Tests for sentence-based detection functionality."""

import pytest
from backend.app.service import (
    splitSentencesAdvanced,
    buildSentenceBlocks,
    compareDualModeResults,
)


def test_splitSentencesAdvanced_basic():
    """Test basic sentence splitting."""
    text = "这是第一句。这是第二句！这是第三句？"
    sentences = splitSentencesAdvanced(text)
    
    assert len(sentences) == 3
    assert sentences[0]["text"] == "这是第一句。"
    assert sentences[1]["text"] == "这是第二句！"
    assert sentences[2]["text"] == "这是第三句？"
    assert sentences[0]["start"] == 0
    assert sentences[0]["end"] > 0


def test_splitSentencesAdvanced_with_ellipsis():
    """Test sentence splitting with ellipsis."""
    text = "他说……这不对。然后离开了。"
    sentences = splitSentencesAdvanced(text)
    
    assert len(sentences) >= 2
    # Ellipsis should be handled correctly
    

def test_splitSentencesAdvanced_decimal_numbers():
    """Test that decimal numbers are not split."""
    text = "价格是1.5元。数量是2.3个。"
    sentences = splitSentencesAdvanced(text)
    
    assert len(sentences) == 2
    assert "1.5" in sentences[0]["text"]
    assert "2.3" in sentences[1]["text"]


def test_splitSentencesAdvanced_english():
    """Test English sentence splitting."""
    text = "This is the first sentence. This is the second sentence! Dr. Smith said this."
    sentences = splitSentencesAdvanced(text)
    
    # Dr. should not be treated as sentence ending
    assert any("Dr. Smith" in s["text"] for s in sentences)


def test_buildSentenceBlocks_short_sentences():
    """Test that short sentences are aggregated."""
    text = "短句。很短。也短。这些应该聚合。"
    blocks = buildSentenceBlocks(text, min_chars=10, target_chars=50)
    
    # Should have fewer blocks than sentences due to aggregation
    sentences = splitSentencesAdvanced(text)
    assert len(blocks) < len(sentences)


def test_buildSentenceBlocks_long_sentence():
    """Test that long sentences remain standalone."""
    long_sentence = "这是一个非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常长的句子。"
    text = long_sentence + "短句。"
    
    blocks = buildSentenceBlocks(text, target_chars=100, max_chars=200)
    
    # Long sentence should be its own block
    assert len(blocks) >= 1
    assert any(len(b["text"]) > 200 for b in blocks)


def test_buildSentenceBlocks_structure():
    """Test that sentence blocks have correct structure."""
    text = "第一句。第二句。第三句。"
    blocks = buildSentenceBlocks(text)
    
    for block in blocks:
        assert "index" in block
        assert "label" in block
        assert block["label"] == "sentence_block"
        assert "needDetect" in block
        assert block["needDetect"] is True
        assert "start" in block
        assert "end" in block
        assert "text" in block
        assert "sentenceCount" in block


def test_compareDualModeResults_basic():
    """Test basic comparison of dual mode results."""
    para_segments = [
        {
            "chunkId": 0,
            "offsets": {"start": 0, "end": 100},
            "aiProbability": 0.6
        },
        {
            "chunkId": 1,
            "offsets": {"start": 100, "end": 200},
            "aiProbability": 0.4
        }
    ]
    
    sent_segments = [
        {
            "chunkId": 0,
            "offsets": {"start": 0, "end": 50},
            "aiProbability": 0.65
        },
        {
            "chunkId": 1,
            "offsets": {"start": 50, "end": 100},
            "aiProbability": 0.58
        },
        {
            "chunkId": 2,
            "offsets": {"start": 100, "end": 200},
            "aiProbability": 0.42
        }
    ]
    
    text = "a" * 200
    comparison = compareDualModeResults(para_segments, sent_segments, text)
    
    assert "probabilityDiff" in comparison
    assert "consistencyScore" in comparison
    assert "divergentRegions" in comparison
    assert isinstance(comparison["probabilityDiff"], float)
    assert 0 <= comparison["consistencyScore"] <= 1


def test_compareDualModeResults_empty():
    """Test comparison with empty segments."""
    comparison = compareDualModeResults([], [], "")
    
    assert comparison["probabilityDiff"] == 0.0
    assert comparison["consistencyScore"] == 1.0
    assert len(comparison["divergentRegions"]) == 0


def test_compareDualModeResults_divergent_regions():
    """Test detection of divergent regions."""
    para_segments = [
        {
            "chunkId": 0,
            "offsets": {"start": 0, "end": 100},
            "aiProbability": 0.8  # High AI probability
        }
    ]
    
    sent_segments = [
        {
            "chunkId": 0,
            "offsets": {"start": 0, "end": 100},
            "aiProbability": 0.3  # Low AI probability
        }
    ]
    
    text = "This is a test text with some content here for testing purposes and more text to make it long enough."
    comparison = compareDualModeResults(para_segments, sent_segments, text, diff_threshold=0.20)
    
    # Should detect divergent region due to 0.5 difference
    assert len(comparison["divergentRegions"]) > 0
    assert comparison["divergentRegions"][0]["probabilityDiff"] > 0.2
