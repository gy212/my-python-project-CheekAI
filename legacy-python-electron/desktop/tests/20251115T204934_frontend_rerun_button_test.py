"""前端 重新检测 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="frontend_rerun_button",
    category="frontend",
    description="模拟点击重新检测按钮——再次检测并刷新历史",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="rerun_detect",
            method="POST",
            path="/api/detect",
            description="重新向后端提交检测请求",
            json_body={
                "text": "重新检测流程自动化测试。",
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
        StepConfig(
            name="refresh_history",
            method="GET",
            path="/api/history/list",
            description="重新检测成功后刷新历史列表"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
