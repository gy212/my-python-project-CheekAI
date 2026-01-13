"""后端 PATCH /api/config/file/{path} 测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="config_file_patch",
    category="backend",
    description="PATCH /api/config/file/{path} 局部更新",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="patch_value",
            method="PATCH",
            path="/api/config/file/tests.patchKey",
            description="写入 patchKey",
            json_body={"value": {"enabled": True, "ts": "2025-11-15"}},
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
