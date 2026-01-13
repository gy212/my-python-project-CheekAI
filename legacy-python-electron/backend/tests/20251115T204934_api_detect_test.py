"""后端 /api/detect 检测流程测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

SAMPLE_TEXT = "这是一段用于自动化测试的文本，包含多个句子，方便 CheekAI 进行检测。"

scenario = ApiScenario(
    functionality="api_detect",
    category="backend",
    description="POST /api/detect 触发一次完整检测",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="submit_detect",
            method="POST",
            path="/api/detect",
            description="提交检测文本",
            json_body={
                "text": SAMPLE_TEXT,
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
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
