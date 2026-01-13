"""后端 /api/rubric/changelog 变更日志测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_rubric_changelog",
    category="backend",
    description="GET /api/rubric/changelog 查看历史记录",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="rubric_changelog",
            method="GET",
            path="/api/rubric/changelog",
            description="拉取评分规则变更历史"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
