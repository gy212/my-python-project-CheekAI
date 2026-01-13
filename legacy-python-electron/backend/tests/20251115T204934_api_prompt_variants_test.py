"""后端 /api/prompt/variants 提示模板列表测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_prompt_variants",
    category="backend",
    description="GET /api/prompt/variants 拉取提示模版",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="prompt_variants",
            method="GET",
            path="/api/prompt/variants",
            description="查询提示模版列表"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
