"""后端 /api/config/file 读取配置测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="config_file_get",
    category="backend",
    description="GET /api/config/file 拉取配置文件",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="fetch_config",
            method="GET",
            path="/api/config/file",
            description="读取当前配置"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
