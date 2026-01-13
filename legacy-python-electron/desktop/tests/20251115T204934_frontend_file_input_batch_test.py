"""前端 文件选择（多文件批量检测）事件测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

BATCH_ITEMS = [
    {
        "id": "ui-file-a",
        "text": "当用户选择多个文件时，前端需要依次预处理并批量检测。",
        "language": "zh-CN",
        "providers": [],
        "usePerplexity": True,
        "useStylometry": True,
        "chunking": {"chunkSizeTokens": 400, "overlapTokens": 80},
        "preprocessOptions": {
            "autoLanguage": True,
            "stripHtml": True,
            "redactPII": False,
            "normalizePunctuation": True,
            "chunkSizeTokens": 400,
            "overlapTokens": 80,
        },
        "sensitivity": "medium",
    },
    {
        "id": "ui-file-b",
        "text": "此脚本模拟 change 事件触发批量检测逻辑。",
        "language": "zh-CN",
        "providers": [],
        "usePerplexity": True,
        "useStylometry": True,
        "chunking": {"chunkSizeTokens": 400, "overlapTokens": 80},
        "preprocessOptions": {
            "autoLanguage": True,
            "stripHtml": True,
            "redactPII": False,
            "normalizePunctuation": True,
            "chunkSizeTokens": 400,
            "overlapTokens": 80,
        },
        "sensitivity": "medium",
    },
]

scenario = ApiScenario(
    functionality="frontend_file_input_batch",
    category="frontend",
    description="模拟 fileInput change 事件导致的批量检测请求",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="batch_from_ui",
            method="POST",
            path="/api/detect/batch",
            description="发送多文件批量检测任务",
            json_body={"items": BATCH_ITEMS, "parallel": 2},
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
