"""后端 GET /api/config/file/versions 版本列表测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="config_file_versions",
    category="backend",
    description="GET /api/config/file/versions 获取历史版本",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="list_versions",
            method="GET",
            path="/api/config/file/versions",
            description="列出可回滚的版本"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
