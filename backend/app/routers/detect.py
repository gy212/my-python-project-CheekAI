# -*- coding: utf-8 -*-
"""Detection routes - /api/detect, /api/detect/batch, /api/paper/analyze, etc."""

import asyncio
import logging
import uuid
from typing import List

from fastapi import APIRouter, HTTPException

from ..schemas import (
    DetectRequest,
    DetectResponse,
    BatchDetectRequest,
    BatchDetectResponse,
    BatchItemResponse,
    BatchSummary,
    CalibrateRequest,
    CalibrateResponse,
    PromptVariantsResponse,
    PromptVariant,
    ConsistencyCheckRequest,
    ConsistencyCheckResponse,
    PaperAnalyzeRequest,
    PaperAnalyzeResponse,
    ReadabilityScores,
    MultiRoundSummary,
    MultiRoundDetail,
    SuggestionItem,
    AggregationResponse,
    AggregationThresholds,
)
from ..service import (
    detect_async,
    setCalibration,
    applyCalibration,
    getRubricInfo,
    DEFAULT_BUFFER_MARGIN,
    deriveDecision,
    getPromptVariants,
    paper_analyze,
    _calibration,
    _rubric_version,
    _rubric_changelog,
)
from ..providers import getGLMKey
from ..services.response_builder import (
    build_detect_response,
    build_batch_item_response,
    build_aggregation_response,
)

router = APIRouter()


@router.post("/api/detect", response_model=DetectResponse)
async def post_detect(req: DetectRequest):
    """Single text detection endpoint."""
    try:
        logging.info(f"detect_request len={len(req.text or '')} providers={req.providers}")
    except Exception:
        pass

    try:
        agg, segments, pre_summary, cost = await detect_async(
            req.text,
            req.language or "zh-CN",
            req.chunking.chunkSizeTokens,
            req.chunking.overlapTokens,
            req.providers,
            req.genre,
            req.usePerplexity,
            req.useStylometry,
            req.sensitivity,
        )
    except Exception as exc:
        logging.exception("detect_internal_error")
        raise HTTPException(status_code=502, detail={"code": "detect_failed", "message": str(exc)})

    # Apply calibration
    agg["overallProbability"] = applyCalibration(agg["overallProbability"])
    agg["decision"] = deriveDecision(
        agg["overallProbability"],
        agg["thresholds"],
        float(agg.get("bufferMargin", DEFAULT_BUFFER_MARGIN))
    )
    for s in segments:
        s["aiProbability"] = applyCalibration(s["aiProbability"])

    return build_detect_response(agg, segments, pre_summary, cost)


@router.post("/api/detect/batch", response_model=BatchDetectResponse)
async def post_detect_batch(req: BatchDetectRequest):
    """Batch text detection endpoint."""
    if not req.items:
        raise HTTPException(status_code=400, detail={"code": "empty_batch", "message": "至少需要 1 条待检测任务"})

    try:
        logging.info(f"batch_detect_request count={len(req.items)} providers={[item.providers for item in req.items]}")
    except Exception:
        pass

    sem_count = max(1, int(req.parallel or 4))
    sem = asyncio.Semaphore(sem_count)

    async def run_item(it) -> BatchItemResponse:
        async with sem:
            agg, segments, pre_summary, cost = await detect_async(
                it.text,
                it.language or "zh-CN",
                it.chunking.chunkSizeTokens,
                it.chunking.overlapTokens,
                it.providers,
                it.genre,
                it.usePerplexity,
                it.useStylometry,
                it.sensitivity,
            )
            # Apply calibration
            agg["overallProbability"] = applyCalibration(agg["overallProbability"])
            agg["decision"] = deriveDecision(
                agg["overallProbability"],
                agg["thresholds"],
                float(agg.get("bufferMargin", DEFAULT_BUFFER_MARGIN))
            )
            for s in segments:
                s["aiProbability"] = applyCalibration(s["aiProbability"])

            return build_batch_item_response(it.id, agg, segments, pre_summary, cost)

    tasks = [run_item(it) for it in req.items]
    done = await asyncio.gather(*tasks, return_exceptions=True)

    items: List[BatchItemResponse] = []
    probs: List[float] = []
    fails = 0

    for r in done:
        if isinstance(r, Exception):
            fails += 1
            continue
        items.append(r)
        probs.append(r.aggregation.overallProbability)

    probs.sort()
    avg = sum(probs) / max(1, len(probs)) if probs else 0.0
    idx = int(max(0, min(len(probs) - 1, round(0.95 * (len(probs) - 1))))) if probs else 0
    p95 = probs[idx] if probs else 0.0

    summary = BatchSummary(count=len(req.items), failCount=fails, avgProbability=avg, p95Probability=p95)
    return BatchDetectResponse(items=items, summary=summary)


@router.post("/api/calibrate", response_model=CalibrateResponse)
def post_calibrate(req: CalibrateRequest):
    """Calibration endpoint."""
    items = [{"prob": float(it.prob), "label": int(it.label)} for it in req.items]
    setCalibration(items)
    cur = _calibration.get("current", _rubric_version)
    cfg = _calibration.get("byVersion", {}).get(cur, {})
    return CalibrateResponse(ok=True, version=cur, A=float(cfg.get("A", 0.0)), B=float(cfg.get("B", 0.0)))


@router.get("/api/rubric")
def get_rubric():
    """Get rubric info."""
    return getRubricInfo()


@router.get("/api/rubric/changelog")
def get_rubric_changelog():
    """Get rubric changelog."""
    return {"items": _rubric_changelog}


@router.get("/api/prompt/variants", response_model=PromptVariantsResponse)
def get_prompt_variants():
    """Get prompt variants."""
    items = [PromptVariant(**x) for x in getPromptVariants()]
    return PromptVariantsResponse(items=items)


@router.post("/api/consistency/check", response_model=ConsistencyCheckResponse)
def post_consistency_check(req: ConsistencyCheckRequest):
    """Check consistency of segments."""
    segs = req.segments or []
    issues = []
    for s in segs:
        ex = " ".join(s.explanations or [])
        if ("ttr" in ex and "多样性高" in ex) and ("ttr" in ex and "多样性低" in ex):
            issues.append({"segmentId": s.chunkId, "type": "lexical_diversity_conflict", "message": "词汇多样性解释存在矛盾"})
    return ConsistencyCheckResponse(ok=True, issues=[type("ConsistencyIssue", (), x) for x in issues])


@router.post("/api/paper/analyze", response_model=PaperAnalyzeResponse)
async def post_paper_analyze(req: PaperAnalyzeRequest):
    """Paper analysis endpoint."""
    readability, summary = await paper_analyze(
        req.text,
        req.language or "zh-CN",
        req.genre,
        int(req.rounds or 3),
        bool(req.useLLM)
    )

    agg, segments, pre_summary, cost = await detect_async(
        req.text,
        req.language or "zh-CN",
        1500,
        150,
        ["glm:glm-4.6"] if bool(req.useLLM) and getGLMKey() else [],
        req.genre,
        True,
        True,
        "medium",
    )

    agg["overallProbability"] = applyCalibration(agg["overallProbability"])
    agg["decision"] = deriveDecision(
        agg["overallProbability"],
        agg["thresholds"],
        float(agg.get("bufferMargin", DEFAULT_BUFFER_MARGIN))
    )

    suggestions = []
    if readability["clarity"] < 0.6:
        suggestions.append({"title": "提升结构清晰度", "detail": "增加小标题与列表条目，优化段落长度与层次结构。"})
    if readability["fluency"] < 0.6:
        suggestions.append({"title": "增强语言流畅度", "detail": "丰富句式与连接词，减少重复表达，提高困惑度。"})
    if readability["cohesion"] < 0.6:
        suggestions.append({"title": "加强篇章衔接", "detail": "保持段落主题连续性与术语一致性，避免跳跃式表述。"})

    return PaperAnalyzeResponse(
        aggregation=build_aggregation_response(agg),
        readability=ReadabilityScores(**readability),
        multiRound=MultiRoundSummary(
            rounds=summary["rounds"],
            avgProbability=summary["avgProbability"],
            avgConfidence=summary["avgConfidence"],
            variance=summary["variance"],
            details=[MultiRoundDetail(**d) for d in summary["details"]],
            trimmedAvgProbability=summary.get("trimmedAvgProbability"),
            trimmedAvgConfidence=summary.get("trimmedAvgConfidence"),
        ),
        suggestions=[SuggestionItem(**s) for s in suggestions],
    )
