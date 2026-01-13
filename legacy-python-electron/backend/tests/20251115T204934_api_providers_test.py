"""后端 /api/providers 测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_providers",
    category="backend",
    description="GET /api/providers 列出可用 LLM",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="list_providers",
            method="GET",
            path="/api/providers",
            description="查询当前可用模型"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
