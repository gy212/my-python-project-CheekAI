"""前端 导出CSV 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="frontend_export_csv_button",
    category="frontend",
    description="准备检测结果以供导出 CSV",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="prepare_csv",
            method="POST",
            path="/api/detect",
            description="生成 CSV 所需的分段检测数据",
            json_body={
                "text": "导出 CSV 前需要完成检测并获得 segments。",
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
