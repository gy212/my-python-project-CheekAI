"""前端 导出JSON 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="frontend_export_json_button",
    category="frontend",
    description="准备检测结果以供导出 JSON",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="prepare_json",
            method="POST",
            path="/api/detect",
            description="生成可导出的 JSON 检测结果",
            json_body={
                "text": "导出 JSON 流程需要先完成一次检测以缓存结果。",
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
