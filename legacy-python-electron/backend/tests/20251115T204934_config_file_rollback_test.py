"""后端 POST /api/config/file/rollback 测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"


def _remember_version(ctx, _resp, data):
    items = (data or {}).get("items") if isinstance(data, dict) else None
    if isinstance(items, list) and items:
        ctx["rollback_version"] = items[0]


def _build_rollback_body(ctx):
    version = ctx.get("rollback_version", "")
    return {"version": version}


scenario = ApiScenario(
    functionality="config_file_rollback",
    category="backend",
    description="POST /api/config/file/rollback 恢复历史版本",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="create_new_version",
            method="PUT",
            path="/api/config/file",
            description="写入新配置以产生版本",
            json_body={"data": {"tests": {"rollback": "pending"}}},
        ),
        StepConfig(
            name="fetch_versions",
            method="GET",
            path="/api/config/file/versions",
            description="读取最新版本号",
            extract=_remember_version,
        ),
        StepConfig(
            name="rollback",
            method="POST",
            path="/api/config/file/rollback",
            description="执行回滚",
            json_body=_build_rollback_body,
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
