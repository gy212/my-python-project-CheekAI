"""前端 预处理并填充 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig
from tests_shared.utils import load_sample_bytes

GENERATED_AT = "2025-11-15T20:49:34+08:00"
SAMPLE_BYTES = load_sample_bytes()


def _build_files(_ctx):
    return {"file": ("ui_upload.txt", SAMPLE_BYTES, "text/plain")}


scenario = ApiScenario(
    functionality="frontend_upload_button",
    category="frontend",
    description="模拟点击预处理按钮并将返回的文本回填",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="upload_file",
            method="POST",
            path="/api/preprocess/upload",
            description="上传文件以驱动结构化输出",
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
