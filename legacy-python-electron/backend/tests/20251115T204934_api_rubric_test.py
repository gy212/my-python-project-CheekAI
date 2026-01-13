"""后端 /api/rubric 评分标准读取测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_rubric",
    category="backend",
    description="GET /api/rubric 拉取最新评分标准",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="fetch_rubric",
            method="GET",
            path="/api/rubric",
            description="获取 rubric 信息"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
