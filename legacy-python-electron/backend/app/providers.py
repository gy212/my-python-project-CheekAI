import os
import time
import json
import re
from pathlib import Path
from typing import Optional, Dict, Any
import httpx
from .config_store import store
import logging


_JSON_EXTRACT_RE = re.compile(r"\{.*\}", re.DOTALL)


def parseProvider(spec: str) -> Dict[str, str]:
    parts = spec.split(":", 1)
    if len(parts) == 2:
        return {"name": parts[0], "model": parts[1]}
    return {"name": spec, "model": ""}


_GLM_KEY: Optional[str] = None


def setGLMKey(key: Optional[str]) -> None:
    global _GLM_KEY
    k = (key or "").strip()
    _GLM_KEY = k if k else None
    try:
        if k:
            store.set('glm.apiKey', k)
    except Exception:
        pass


def getGLMKey() -> Optional[str]:
    val = store.get('glm.apiKey')
    if val is None:
        val = store.get('deepseek.apiKey')
    if isinstance(val, str):
        v = val.strip()
        if v:
            try:
                logging.debug(f"glm_key_load source=file len={len(v)}")
            except Exception:
                pass
            return v
    if isinstance(_GLM_KEY, str) and _GLM_KEY.strip():
        v = _GLM_KEY.strip()
        try:
            logging.debug(f"glm_key_load source=memory len={len(v)}")
        except Exception:
            pass
        return v
    env1 = os.environ.get("GLM_API_KEY") or os.environ.get("DEEPSEEK_API_KEY")
    if env1 and env1.strip():
        v = env1.strip()
        try:
            logging.debug(f"glm_key_load source=env(primary) len={len(v)}")
        except Exception:
            pass
        return v
    env2 = os.environ.get("CHEEKAI_GLM_API_KEY") or os.environ.get("CHEEKAI_DEEPSEEK_API_KEY")
    if env2 and env2.strip():
        v = env2.strip()
        try:
            logging.debug(f"glm_key_load source=env(secondary) len={len(v)}")
        except Exception:
            pass
        return v
    try:
        logging.debug("glm_key_load source=none")
    except Exception:
        pass
    return None


async def callGLMChat(
    model: str,
    api_key: str,
    system: str,
    user: str,
    max_tokens: int = 2048,
    enable_reasoning: bool = True,
    reasoning_effort: str = "high",
    client: Optional[httpx.AsyncClient] = None,
    retry_on_empty: bool = True,
) -> Optional[Dict[str, Any]]:
    headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
    payload = {
        "model": model,
        "response_format": {"type": "json_object"},
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "max_tokens": max_tokens,
        "temperature": 0.0,
    }
    if enable_reasoning:
        payload["reasoning"] = {"effort": reasoning_effort}
    url = os.environ.get("GLM_API_URL", "https://open.bigmodel.cn/api/paas/v4/chat/completions")
    close_client = False
    if client is None:
        client = httpx.AsyncClient(timeout=80)
        close_client = True
    started = time.time()
    try:
        try:
            r = await client.post(url, headers=headers, json=payload)
        except Exception as exc:
            logging.exception("glm_call_failed_request")
            raise RuntimeError(f"GLM 调用失败: {exc}") from exc
        latency_ms = int((time.time() - started) * 1000)
        try:
            body_preview = r.text[:200]
        except Exception:
            body_preview = ""
        if r.status_code != 200:
            logging.error("glm_call_http_error status=%s latency_ms=%s body=%s", r.status_code, latency_ms, body_preview)
            raise RuntimeError(f"GLM HTTP error {r.status_code}")
        try:
            data = r.json()
        except Exception as exc:
            logging.error("glm_call_invalid_json latency_ms=%s body=%s", latency_ms, body_preview)
            raise RuntimeError("GLM response JSON parse failed") from exc
        content = None
        reasoning = None
        try:
            choice = data.get("choices", [{}])[0]
            message = choice.get("message", {}) if isinstance(choice, dict) else {}
            content = message.get("content")
            reasoning = message.get("reasoning_content") or data.get("reasoning_content")
        except Exception:
            content = None
        if not content and reasoning:
            # 尝试从 reasoning_content 中提取 JSON 片段；若无法提取则继续走 retry 分支
            m = _JSON_EXTRACT_RE.search(reasoning)
            content = m.group(0) if m else None
        if not content and retry_on_empty:
            logging.error("glm_call_missing_content latency_ms=%s body=%s -> retry_without_reasoning", latency_ms, body_preview)
            return await callGLMChat(
                model,
                api_key,
                system,
                user,
                max_tokens=max_tokens,
                enable_reasoning=False,
                reasoning_effort=reasoning_effort,
                client=client if not close_client else None,
                retry_on_empty=False,
            )
        if not content:
            logging.error("glm_call_missing_content latency_ms=%s body=%s", latency_ms, body_preview)
            raise RuntimeError("GLM 返回缺少内容")
        try:
            logging.info("glm_call_ok model=%s latency_ms=%s", model, latency_ms)
        except Exception:
            pass
        result = {"content": content, "raw": data, "latency_ms": latency_ms}
        if reasoning:
            result["reasoning"] = reasoning
        try:
            log_dir = Path(__file__).resolve().parents[1] / "logs"
            log_dir.mkdir(parents=True, exist_ok=True)
            log_path = log_dir / "glm_last_response.json"
            log_payload = {
                "ts": time.strftime("%Y-%m-%dT%H:%M:%S", time.localtime()),
                "model": model,
                "latencyMs": latency_ms,
                "response": data,
            }
            with open(log_path, "w", encoding="utf-8") as f:
                json.dump(log_payload, f, ensure_ascii=False, indent=2)
        except Exception as log_exc:
            try:
                logging.debug("glm_log_write_failed %s", log_exc)
            except Exception:
                pass
        return result
    finally:
        if close_client:
            await client.aclose()


def getDeepSeekKey() -> Optional[str]:
    """
    Backwards-compatible shim so legacy bytecode can still import the old helper.
    """
    return getGLMKey()


async def callDeepSeekChat(
    model: str,
    api_key: str,
    system: str,
    user: str,
    max_tokens: int = 256,
    client: Optional[httpx.AsyncClient] = None,
) -> Optional[Dict[str, Any]]:
    """
    DeepSeek was replaced by GLM; reuse the GLM call path so cached bytecode keeps working.
    """
    return await callGLMChat(model, api_key, system, user, max_tokens=max_tokens, client=client)
