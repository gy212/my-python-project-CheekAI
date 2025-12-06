# -*- coding: utf-8 -*-
"""Shared utilities for building API responses."""

import uuid
from typing import Dict, Any, List

from ..schemas import (
    DetectResponse,
    AggregationResponse,
    SegmentResponse,
    PreprocessSummary,
    CostBreakdown,
    AggregationThresholds,
    BatchItemResponse,
)
from ..service import DEFAULT_BUFFER_MARGIN
from ..core.config import API_VERSION


def build_aggregation_response(agg: Dict[str, Any]) -> AggregationResponse:
    """Build AggregationResponse from aggregation dict."""
    return AggregationResponse(
        overallProbability=agg["overallProbability"],
        overallConfidence=agg["overallConfidence"],
        method=agg["method"],
        thresholds=AggregationThresholds(**agg["thresholds"]),
        rubricVersion=agg["rubricVersion"],
        decision=agg["decision"],
        bufferMargin=float(agg.get("bufferMargin", DEFAULT_BUFFER_MARGIN)),
        stylometryProbability=float(agg.get("stylometryProbability", agg["overallProbability"])),
        qualityScoreNormalized=float(agg.get("qualityScoreNormalized", 0.0)),
        blockWeights=dict(agg.get("blockWeights", {})) if isinstance(agg.get("blockWeights", {}), dict) else None,
        dimensionScores=dict(agg.get("dimensionScores", {})) if isinstance(agg.get("dimensionScores", {}), dict) else None,
    )


def build_segments_response(segments: List[Dict[str, Any]]) -> List[SegmentResponse]:
    """Build list of SegmentResponse from segments list."""
    return [
        SegmentResponse(
            chunkId=s["chunkId"],
            language=s["language"],
            offsets=s["offsets"],
            aiProbability=s["aiProbability"],
            confidence=s["confidence"],
            signals=s["signals"],
            explanations=s["explanations"],
        )
        for s in segments
    ]


def build_detect_response(
    agg: Dict[str, Any],
    segments: List[Dict[str, Any]],
    pre_summary: Dict[str, Any],
    cost: Dict[str, Any],
) -> DetectResponse:
    """Build complete DetectResponse."""
    return DetectResponse(
        aggregation=build_aggregation_response(agg),
        segments=build_segments_response(segments),
        preprocessSummary=PreprocessSummary(**pre_summary),
        cost=CostBreakdown(**cost),
        version=API_VERSION,
        requestId=str(uuid.uuid4()),
    )


def build_batch_item_response(
    item_id: str,
    agg: Dict[str, Any],
    segments: List[Dict[str, Any]],
    pre_summary: Dict[str, Any],
    cost: Dict[str, Any],
) -> BatchItemResponse:
    """Build BatchItemResponse for batch detection."""
    return BatchItemResponse(
        id=item_id,
        aggregation=build_aggregation_response(agg),
        segments=build_segments_response(segments),
        preprocessSummary=PreprocessSummary(**pre_summary),
        cost=CostBreakdown(**cost),
        version=API_VERSION,
    )
