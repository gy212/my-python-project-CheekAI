"""后端 DELETE /api/config/file/{path} 删除配置测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="config_file_delete",
    category="backend",
    description="DELETE /api/config/file/{path} 移除特定键",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="prepare_key",
            method="PATCH",
            path="/api/config/file/tests.toDelete",
            description="先写入将被删除的键",
            json_body={"value": "temp-value"},
        ),
        StepConfig(
            name="delete_key",
            method="DELETE",
            path="/api/config/file/tests.toDelete",
            description="删除 tests.toDelete",
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
