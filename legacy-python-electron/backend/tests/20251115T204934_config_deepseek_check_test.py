"""后端 /api/config/glm/check 测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="config_glm_check",
    category="backend",
    description="GET /api/config/glm/check 查看密钥状态",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="check_key",
            method="GET",
            path="/api/config/glm/check",
            description="确认是否已保存API Key"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
