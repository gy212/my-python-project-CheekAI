"""前端 复制到文本框 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig
from tests_shared.utils import load_sample_bytes

GENERATED_AT = "2025-11-15T20:49:34+08:00"
SAMPLE_BYTES = load_sample_bytes()


def _files(_ctx):
    return {"file": ("copy_structured.txt", SAMPLE_BYTES, "text/plain")}


def _mark_text(ctx, _resp, data):
    if isinstance(data, dict):
        text_value = data.get("formattedText") or data.get("normalizedText") or ""
        ctx["formatted_length"] = len(text_value)


scenario = ApiScenario(
    functionality="frontend_copy_structured_button",
    category="frontend",
    description="预处理后准备格式化文本，供复制按钮注入至输入框",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="prefetch_formatted",
            method="POST",
            path="/api/preprocess/upload",
            description="上传样例文档，拿到 formattedText",
            data_body={
                "autoLanguage": "true",
                "stripHtml": "true",
                "redactPII": "false",
                "normalizePunctuationOpt": "true",
                "chunkSizeTokens": "400",
                "overlapTokens": "80",
            },
            files=_files,
            extract=_mark_text,
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
