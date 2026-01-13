# -*- coding: utf-8 -*-
"""Configuration routes - /api/config/*, /api/providers."""

import logging
from typing import Dict, Any

from fastapi import APIRouter, HTTPException, UploadFile, File, Form
from pydantic import BaseModel

from ..providers import getGLMKey, setGLMKey
from ..config_store import store
from ..preprocess import preprocess_upload_file
from ..schemas import PreprocessUploadResponse, PreprocessSummary, SegmentResponse

router = APIRouter()


class GLMKeyBody(BaseModel):
    """Request body for GLM API key."""
    apiKey: str


@router.get("/api/health")
def health():
    """Health check endpoint."""
    return {"status": "ok"}


@router.get("/api/providers")
def get_providers():
    """Get available providers."""
    items = []
    if getGLMKey():
        items.append({"name": "glm", "models": ["glm-4.6"], "rateLimit": "per account"})
    return {"items": items}


@router.post("/api/config/glm")
def post_glm_key(body: GLMKeyBody):
    """Set GLM API key."""
    k = (body.apiKey or "").strip()
    if not k:
        raise HTTPException(status_code=400, detail={"code": "invalid_api_key", "message": "apiKey 不能为空"})
    try:
        setGLMKey(k)
        logging.info(f"glm_key_set len={len(k)}")
        return {"ok": True}
    except Exception as e:
        logging.error(f"glm_key_set_fail err={e}")
        raise HTTPException(status_code=500, detail={"code": "set_failed", "message": "保存失败"})


@router.get("/api/config/glm/check")
def get_glm_check():
    """Check if GLM key is present."""
    present = True if getGLMKey() else False
    return {"present": present}


@router.get("/api/config/file")
def get_config_file():
    """Get config file contents."""
    return store.load()


@router.put("/api/config/file")
def put_config_file(body: Dict[str, Any]):
    """Replace config file contents."""
    cfg = store.load()
    if not isinstance(body, dict):
        raise HTTPException(status_code=400, detail={"code": "invalid_body", "message": "需为JSON对象"})
    cfg["data"] = body.get("data", body)
    store.save(cfg)
    return {"ok": True}


@router.patch("/api/config/file/{path:path}")
def patch_config_file(path: str, body: Dict[str, Any]):
    """Patch config file at path."""
    if "value" not in body:
        raise HTTPException(status_code=400, detail={"code": "invalid_body", "message": "缺少value"})
    store.set(path, body.get("value"))
    return {"ok": True}


@router.delete("/api/config/file/{path:path}")
def delete_config_file(path: str):
    """Delete config at path."""
    ok = store.delete(path)
    if not ok:
        raise HTTPException(status_code=404, detail={"code": "not_found", "message": "不存在"})
    return {"ok": True}


@router.get("/api/config/file/versions")
def get_config_versions():
    """Get config versions."""
    return {"items": store.versions()}


@router.post("/api/config/file/rollback")
def post_config_rollback(body: Dict[str, Any]):
    """Rollback config to version."""
    ts = str(body.get("version", "")).strip()
    if not ts:
        raise HTTPException(status_code=400, detail={"code": "invalid_version", "message": "缺少version"})
    ok = store.rollback(ts)
    if not ok:
        raise HTTPException(status_code=404, detail={"code": "version_not_found", "message": "版本不存在"})
    return {"ok": True}


@router.post("/api/preprocess/upload", response_model=PreprocessUploadResponse)
async def preprocess_upload(
    file: UploadFile = File(...),
    autoLanguage: bool = Form(True),
    stripHtml: bool = Form(True),
    redactPII: bool = Form(False),
    normalizePunctuationOpt: bool = Form(True),
    chunkSizeTokens: int = Form(400),
    overlapTokens: int = Form(80),
):
    """Upload and preprocess file."""
    result = await preprocess_upload_file(
        file,
        normalize_punctuation=normalizePunctuationOpt,
        auto_language=autoLanguage,
        chunk_size_tokens=chunkSizeTokens,
        overlap_tokens=overlapTokens,
    )
    return PreprocessUploadResponse(
        normalizedText=result["normalizedText"],
        preprocessSummary=PreprocessSummary(**result["preprocessSummary"]),
        segments=[
            SegmentResponse(
                chunkId=s["chunkId"],
                language=s["language"],
                offsets=s["offsets"],
                aiProbability=s["aiProbability"],
                confidence=s["confidence"],
                signals=s["signals"],
                explanations=s["explanations"],
            )
            for s in result["segments"]
        ],
        structuredNodes=result["structuredNodes"],
        formattedText=result["formattedText"],
        formatSummary=result["formatSummary"],
        mapping=result["mapping"],
        comparison=result["comparison"],
    )
