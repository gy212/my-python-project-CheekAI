# -*- coding: utf-8 -*-
"""History and review routes - /api/history/*, /api/review/*."""

import time
from typing import Dict, Any

from fastapi import APIRouter

from ..schemas import (
    HistorySaveRequest,
    HistorySaveResponse,
    HistoryListResponse,
    HistoryItem,
    AggregationResponse,
    MultiRoundSummary,
    ReviewSubmitRequest,
    ReviewSubmitResponse,
    ReviewSummaryResponse,
)
from ..config_store import store

router = APIRouter()


@router.post("/api/history/save", response_model=HistorySaveResponse)
def post_history_save(req: HistorySaveRequest):
    """Save history item."""
    item = {
        "id": req.id,
        "ts": time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime()),
        "reqParams": req.reqParams,
        "aggregation": req.aggregation.dict(),
        "multiRound": req.multiRound.dict() if req.multiRound else None,
    }
    items = store.get("paper.history") or []
    items.insert(0, item)
    store.set("paper.history", items[:100])
    return HistorySaveResponse(ok=True, total=len(items))


@router.get("/api/history/list", response_model=HistoryListResponse)
def get_history_list():
    """List history items."""
    items = store.get("paper.history") or []

    def map_item(x: Dict[str, Any]) -> HistoryItem:
        return HistoryItem(
            id=str(x.get("id", "")),
            ts=str(x.get("ts", "")),
            reqParams=dict(x.get("reqParams", {})),
            aggregation=AggregationResponse(**x.get("aggregation", {})),
            multiRound=MultiRoundSummary(**x.get("multiRound", {})) if x.get("multiRound") else None,
        )

    return HistoryListResponse(items=[map_item(x) for x in items])


@router.post("/api/review/submit", response_model=ReviewSubmitResponse)
def post_review_submit(req: ReviewSubmitRequest):
    """Submit review."""
    cfg = store.load()
    logs = cfg.get("data", {}).get("review", {}).get("logs", [])
    item = {
        "ts": time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime()),
        "requestId": req.requestId,
        "overallProbability": float(req.overallProbability),
        "overallConfidence": float(req.overallConfidence),
        "decision": str(req.decision),
        "label": int(req.label) if req.label is not None else None,
        "notes": req.notes or "",
    }
    logs.append(item)
    store.set("review.logs", logs)
    passCount = sum(1 for x in logs if x.get("decision") == "pass")
    reviewCount = sum(1 for x in logs if x.get("decision") == "review")
    flagCount = sum(1 for x in logs if x.get("decision") == "flag")
    return ReviewSubmitResponse(
        ok=True,
        total=len(logs),
        passCount=passCount,
        reviewCount=reviewCount,
        flagCount=flagCount
    )


@router.get("/api/review/summary", response_model=ReviewSummaryResponse)
def get_review_summary():
    """Get review summary with metrics."""
    logs = store.get("review.logs") or []
    labeled = [x for x in logs if x.get("label") in (0, 1) and x.get("decision") in ("pass", "flag")]

    tp = sum(1 for x in labeled if x.get("decision") == "flag" and x.get("label") == 1)
    tn = sum(1 for x in labeled if x.get("decision") == "pass" and x.get("label") == 0)
    fp = sum(1 for x in labeled if x.get("decision") == "flag" and x.get("label") == 0)
    fn = sum(1 for x in labeled if x.get("decision") == "pass" and x.get("label") == 1)

    denom = max(1, len(labeled))
    accuracy = (tp + tn) / denom
    precision = tp / max(1, (tp + fp))
    recall = tp / max(1, (tp + fn))
    f1 = (2 * precision * recall) / max(1e-6, (precision + recall))

    return ReviewSummaryResponse(
        total=len(logs),
        labeled=len(labeled),
        tp=tp,
        tn=tn,
        fp=fp,
        fn=fn,
        accuracy=accuracy,
        precision=precision,
        recall=recall,
        f1=f1
    )
