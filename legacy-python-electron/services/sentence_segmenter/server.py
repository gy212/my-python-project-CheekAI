# -*- coding: utf-8 -*-
"""
Text Segmentation Service
独立的分句/分段微服务，供 Rust Tauri 调用

支持:
    - spaCy: 智能分句
    - wtpsplit/SaT: 智能分段（话题分割）

启动方式:
    python server.py [--port 8788]

API:
    POST /segment          - spaCy 分句
    POST /segment/blocks   - 分句并聚合为块
    POST /paragraph        - wtpsplit 智能分段
    POST /paragraph/blocks - 分段并聚合为块
"""

import argparse
import json
import logging
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Dict, List, Any, Optional
import threading

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("sentence_segmenter")

# 全局模型缓存
_models: Dict[str, Any] = {}
_models_lock = threading.Lock()

# wtpsplit 模型缓存
_wtp_model = None
_wtp_lock = threading.Lock()


def load_model(language: str) -> Any:
    """
    加载 spaCy 模型，支持中英文
    
    模型下载命令:
        python -m spacy download zh_core_web_sm  # 中文
        python -m spacy download en_core_web_sm  # 英文
    """
    model_name = {
        "zh": "zh_core_web_sm",
        "zh-CN": "zh_core_web_sm",
        "zh-TW": "zh_core_web_sm",
        "en": "en_core_web_sm",
        "en-US": "en_core_web_sm",
    }.get(language, "zh_core_web_sm")
    
    with _models_lock:
        if model_name not in _models:
            try:
                import spacy
                logger.info(f"Loading spaCy model: {model_name}")
                _models[model_name] = spacy.load(model_name)
                logger.info(f"Model {model_name} loaded successfully")
            except OSError as e:
                logger.error(f"Failed to load model {model_name}: {e}")
                logger.info(f"Please run: python -m spacy download {model_name}")
                raise
        return _models[model_name]


def load_wtp_model():
    """
    加载 wtpsplit SaT 模型用于分段
    """
    global _wtp_model
    with _wtp_lock:
        if _wtp_model is None:
            try:
                from wtpsplit import SaT
                logger.info("Loading wtpsplit SaT model...")
                # 使用较小的模型以提高速度
                _wtp_model = SaT("sat-3l-sm")
                logger.info("wtpsplit SaT model loaded successfully")
            except Exception as e:
                logger.error(f"Failed to load wtpsplit model: {e}")
                # 回退到旧版 WtP
                try:
                    from wtpsplit import WtP
                    logger.info("Falling back to WtP model...")
                    _wtp_model = WtP("wtp-bert-mini", ignore_legacy_warning=True)
                    logger.info("WtP model loaded successfully")
                except Exception as e2:
                    logger.error(f"Failed to load WtP model: {e2}")
                    raise
        return _wtp_model


def segment_paragraphs(text: str, language: str = "zh", threshold: float = 0.5) -> List[Dict[str, Any]]:
    """
    使用 wtpsplit 进行智能分段（话题分割）
    
    Args:
        text: 输入文本
        language: 语言代码 (zh, en)
        threshold: 分段阈值 (0-1)，越高分段越少
    
    Returns:
        段落列表，每个包含 text, start, end
    """
    if not text or not text.strip():
        return []
    
    wtp = load_wtp_model()
    
    # wtpsplit 返回分段后的文本列表
    lang_code = "zh" if language.startswith("zh") else "en"
    
    try:
        # SaT 模型使用 split 方法
        paragraphs_text = wtp.split(text, lang_code=lang_code, threshold=threshold)
    except Exception as e:
        logger.warning(f"wtpsplit failed: {e}, falling back to simple split")
        # 回退到简单的空行分割
        paragraphs_text = [p.strip() for p in text.split('\n\n') if p.strip()]
        if not paragraphs_text:
            paragraphs_text = [text]
    
    # 计算每个段落的偏移量
    paragraphs = []
    current_pos = 0
    
    for para_text in paragraphs_text:
        if not para_text:
            continue
        # 查找段落在原文中的位置
        start = text.find(para_text, current_pos)
        if start == -1:
            start = current_pos
        end = start + len(para_text)
        
        paragraphs.append({
            "text": para_text,
            "start": start,
            "end": end,
        })
        current_pos = end
    
    return paragraphs


def segment_paragraphs_to_blocks(
    text: str,
    language: str = "zh",
    threshold: float = 0.5,
    min_chars: int = 100,
    target_chars: int = 500,
    max_chars: int = 1000,
) -> List[Dict[str, Any]]:
    """
    使用 wtpsplit 分段后聚合为检测块
    
    Args:
        text: 输入文本
        language: 语言代码
        threshold: 分段阈值
        min_chars: 最小字符数（短段会合并）
        target_chars: 目标字符数
        max_chars: 最大字符数
    
    Returns:
        块列表
    """
    paragraphs = segment_paragraphs(text, language, threshold)
    if not paragraphs:
        return []
    
    blocks = []
    current_paragraphs = []
    current_chars = 0
    
    for para in paragraphs:
        para_len = len(para["text"])
        
        # 超长段落单独成块
        if para_len > max_chars:
            # 先 flush 当前累积
            if current_paragraphs:
                block_text = "\n\n".join(p["text"] for p in current_paragraphs)
                blocks.append({
                    "index": len(blocks),
                    "label": "paragraph_block",
                    "needDetect": True,
                    "mergeWithPrev": False,
                    "start": current_paragraphs[0]["start"],
                    "end": current_paragraphs[-1]["end"],
                    "text": block_text,
                    "paragraphCount": len(current_paragraphs),
                })
                current_paragraphs = []
                current_chars = 0
            
            # 添加超长段落
            blocks.append({
                "index": len(blocks),
                "label": "paragraph_block",
                "needDetect": True,
                "mergeWithPrev": False,
                "start": para["start"],
                "end": para["end"],
                "text": para["text"],
                "paragraphCount": 1,
            })
            continue
        
        # 尝试添加到当前块
        if current_chars + para_len <= target_chars or current_chars == 0:
            current_paragraphs.append(para)
            current_chars += para_len
        else:
            # 当前块已满，flush
            block_text = "\n\n".join(p["text"] for p in current_paragraphs)
            blocks.append({
                "index": len(blocks),
                "label": "paragraph_block",
                "needDetect": True,
                "mergeWithPrev": False,
                "start": current_paragraphs[0]["start"],
                "end": current_paragraphs[-1]["end"],
                "text": block_text,
                "paragraphCount": len(current_paragraphs),
            })
            
            # 开始新块
            current_paragraphs = [para]
            current_chars = para_len
    
    # 处理剩余
    if current_paragraphs:
        block_text = "\n\n".join(p["text"] for p in current_paragraphs)
        blocks.append({
            "index": len(blocks),
            "label": "paragraph_block",
            "needDetect": True,
            "mergeWithPrev": False,
            "start": current_paragraphs[0]["start"],
            "end": current_paragraphs[-1]["end"],
            "text": block_text,
            "paragraphCount": len(current_paragraphs),
        })
    
    return blocks


def segment_sentences(text: str, language: str = "zh") -> List[Dict[str, Any]]:
    """
    使用 spaCy 进行智能分句
    
    Args:
        text: 输入文本
        language: 语言代码 (zh, en)
    
    Returns:
        句子列表，每个包含 text, start, end
    """
    if not text or not text.strip():
        return []
    
    nlp = load_model(language)
    doc = nlp(text)
    
    sentences = []
    for sent in doc.sents:
        sentences.append({
            "text": sent.text.strip(),
            "start": sent.start_char,
            "end": sent.end_char,
        })
    
    return sentences


def segment_to_blocks(
    text: str,
    language: str = "zh",
    min_chars: int = 50,
    target_chars: int = 200,
    max_chars: int = 300,
) -> List[Dict[str, Any]]:
    """
    分句后聚合为检测块
    
    Args:
        text: 输入文本
        language: 语言代码
        min_chars: 最小字符数（短句会合并）
        target_chars: 目标字符数
        max_chars: 最大字符数（超过会单独成块）
    
    Returns:
        块列表，每个包含 text, start, end, sentence_count
    """
    sentences = segment_sentences(text, language)
    if not sentences:
        return []
    
    blocks = []
    current_sentences = []
    current_chars = 0
    
    for sent in sentences:
        sent_len = len(sent["text"])
        
        # 超长句子单独成块
        if sent_len > max_chars:
            # 先 flush 当前累积
            if current_sentences:
                block_text = " ".join(s["text"] for s in current_sentences)
                blocks.append({
                    "index": len(blocks),
                    "label": "sentence_block",
                    "needDetect": True,
                    "mergeWithPrev": False,
                    "start": current_sentences[0]["start"],
                    "end": current_sentences[-1]["end"],
                    "text": block_text,
                    "sentenceCount": len(current_sentences),
                })
                current_sentences = []
                current_chars = 0
            
            # 添加超长句子
            blocks.append({
                "index": len(blocks),
                "label": "sentence_block",
                "needDetect": True,
                "mergeWithPrev": False,
                "start": sent["start"],
                "end": sent["end"],
                "text": sent["text"],
                "sentenceCount": 1,
            })
            continue
        
        # 尝试添加到当前块
        if current_chars + sent_len <= target_chars or current_chars == 0:
            current_sentences.append(sent)
            current_chars += sent_len
        else:
            # 当前块已满，flush
            block_text = " ".join(s["text"] for s in current_sentences)
            blocks.append({
                "index": len(blocks),
                "label": "sentence_block",
                "needDetect": True,
                "mergeWithPrev": False,
                "start": current_sentences[0]["start"],
                "end": current_sentences[-1]["end"],
                "text": block_text,
                "sentenceCount": len(current_sentences),
            })
            
            # 开始新块
            current_sentences = [sent]
            current_chars = sent_len
    
    # 处理剩余
    if current_sentences:
        block_text = " ".join(s["text"] for s in current_sentences)
        blocks.append({
            "index": len(blocks),
            "label": "sentence_block",
            "needDetect": True,
            "mergeWithPrev": False,
            "start": current_sentences[0]["start"],
            "end": current_sentences[-1]["end"],
            "text": block_text,
            "sentenceCount": len(current_sentences),
        })
    
    return blocks


class SegmentHandler(BaseHTTPRequestHandler):
    """HTTP 请求处理器"""
    
    def log_message(self, format, *args):
        logger.info("%s - %s", self.address_string(), format % args)
    
    def _send_json(self, data: Any, status: int = 200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode("utf-8"))
    
    def _read_json(self) -> Optional[Dict]:
        content_length = int(self.headers.get("Content-Length", 0))
        if content_length == 0:
            return None
        body = self.rfile.read(content_length)
        return json.loads(body.decode("utf-8"))
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()
    
    def do_GET(self):
        """健康检查"""
        if self.path == "/health":
            self._send_json({
                "status": "ok", 
                "service": "text_segmenter",
                "features": ["sentence", "paragraph"]
            })
        else:
            self._send_json({"error": "Not Found"}, 404)
    
    def do_POST(self):
        """处理分句请求"""
        try:
            data = self._read_json()
            if not data:
                self._send_json({"error": "Empty request body"}, 400)
                return
            
            text = data.get("text", "")
            language = data.get("language", "zh")
            
            if self.path == "/segment":
                # 纯分句
                sentences = segment_sentences(text, language)
                self._send_json({"sentences": sentences})
            
            elif self.path == "/segment/blocks":
                # 分句并聚合为块
                min_chars = data.get("minChars", 50)
                target_chars = data.get("targetChars", 200)
                max_chars = data.get("maxChars", 300)
                blocks = segment_to_blocks(
                    text, language, min_chars, target_chars, max_chars
                )
                self._send_json({"blocks": blocks})
            
            elif self.path == "/paragraph":
                # wtpsplit 智能分段
                threshold = data.get("threshold", 0.5)
                paragraphs = segment_paragraphs(text, language, threshold)
                self._send_json({"paragraphs": paragraphs})
            
            elif self.path == "/paragraph/blocks":
                # 分段并聚合为块
                threshold = data.get("threshold", 0.5)
                min_chars = data.get("minChars", 100)
                target_chars = data.get("targetChars", 500)
                max_chars = data.get("maxChars", 1000)
                blocks = segment_paragraphs_to_blocks(
                    text, language, threshold, min_chars, target_chars, max_chars
                )
                self._send_json({"blocks": blocks})
            
            else:
                self._send_json({"error": "Not Found"}, 404)
        
        except Exception as e:
            logger.exception("Request failed")
            self._send_json({"error": str(e)}, 500)


def main():
    parser = argparse.ArgumentParser(description="Text Segmentation Service (spaCy + wtpsplit)")
    parser.add_argument("--port", type=int, default=8788, help="Server port")
    parser.add_argument("--host", default="127.0.0.1", help="Server host")
    parser.add_argument("--preload", default="zh", help="Preload spaCy model for language")
    parser.add_argument("--preload-wtp", action="store_true", help="Preload wtpsplit model")
    args = parser.parse_args()
    
    # 预加载 spaCy 模型
    if args.preload:
        try:
            load_model(args.preload)
        except Exception as e:
            logger.warning(f"Failed to preload spaCy model: {e}")
    
    # 预加载 wtpsplit 模型
    if args.preload_wtp:
        try:
            load_wtp_model()
        except Exception as e:
            logger.warning(f"Failed to preload wtpsplit model: {e}")
    
    server = HTTPServer((args.host, args.port), SegmentHandler)
    logger.info(f"Text Segmenter Service started at http://{args.host}:{args.port}")
    logger.info("Endpoints:")
    logger.info("  /health           - Health check")
    logger.info("  /segment          - spaCy sentence segmentation")
    logger.info("  /segment/blocks   - Sentence blocks")
    logger.info("  /paragraph        - wtpsplit paragraph segmentation")
    logger.info("  /paragraph/blocks - Paragraph blocks")
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        logger.info("Shutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()
