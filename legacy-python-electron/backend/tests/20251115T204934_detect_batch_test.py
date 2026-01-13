"""后端 /api/detect/batch 批量检测测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

SAMPLE_ITEMS = [
    {
        "id": "sample-a",
        "text": "批量检测条目A：这是第一段示例文本。",
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
        "id": "sample-b",
        "text": "批量检测条目B：继续提供另一段文本用于测试。",
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
    functionality="detect_batch",
    category="backend",
    description="POST /api/detect/batch 同时提交多条任务",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="batch_detect",
            method="POST",
            path="/api/detect/batch",
            description="提交 2 条批次检测任务",
            json_body={"items": SAMPLE_ITEMS, "parallel": 2},
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
