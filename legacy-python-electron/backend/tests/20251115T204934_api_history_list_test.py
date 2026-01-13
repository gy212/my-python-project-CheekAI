"""后端 /api/history/list 历史列表测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_history_list",
    category="backend",
    description="GET /api/history/list 拉取最近记录",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="history_list",
            method="GET",
            path="/api/history/list",
            description="列出最新两条检测历史"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
