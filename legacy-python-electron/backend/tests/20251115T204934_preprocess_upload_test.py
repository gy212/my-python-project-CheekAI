"""后端 /api/preprocess/upload 文件预处理测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig
from tests_shared.utils import load_sample_bytes

GENERATED_AT = "2025-11-15T20:49:34+08:00"
SAMPLE_BYTES = load_sample_bytes()


def _build_files(_ctx):
    return {"file": ("test_paragraphs.txt", SAMPLE_BYTES, "text/plain")}


scenario = ApiScenario(
    functionality="preprocess_upload",
    category="backend",
    description="POST /api/preprocess/upload 上传文档触发预处理",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="upload_preprocess",
            method="POST",
            path="/api/preprocess/upload",
            description="上传并获取结构化结果",
            data_body={
                "autoLanguage": "true",
                "stripHtml": "true",
                "redactPII": "false",
                "normalizePunctuationOpt": "true",
                "chunkSizeTokens": "400",
                "overlapTokens": "80",
            },
            files=_build_files,
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
