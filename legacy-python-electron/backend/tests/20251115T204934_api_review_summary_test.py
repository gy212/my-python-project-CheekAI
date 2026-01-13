"""后端 /api/review/summary 复核汇总脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_review_summary",
    category="backend",
    description="GET /api/review/summary 计算标注统计",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="review_summary",
            method="GET",
            path="/api/review/summary",
            description="获取复核准确率指标"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
