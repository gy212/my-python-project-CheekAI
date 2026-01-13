"""后端 /api/calibrate 概率校准测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_calibrate",
    category="backend",
    description="POST /api/calibrate 上传校准样本",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="calibrate",
            method="POST",
            path="/api/calibrate",
            description="提交两条人工标注数据",
            json_body={
                "items": [
                    {"prob": 0.21, "label": 0},
                    {"prob": 0.87, "label": 1},
                ]
            },
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
