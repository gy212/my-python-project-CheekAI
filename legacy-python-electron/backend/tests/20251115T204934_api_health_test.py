"""后端健康检查接口测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_health",
    category="backend",
    description="验证 GET /api/health 返回服务状态",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="health_status",
            method="GET",
            path="/api/health",
            description="健康检查"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
