"""前端 detect 按钮（开始检测）测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="frontend_detect_button",
    category="frontend",
    description="模拟点击检测按钮，向 /api/detect 提交正文",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="ui_detect",
            method="POST",
            path="/api/detect",
            description="前端触发检测请求",
            json_body={
                "text": "桌面端测试：点击开始检测按钮应向后端提交本段文本。",
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
