"""后端 /api/review/submit 人工复核记录脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_review_submit",
    category="backend",
    description="POST /api/review/submit 写入一条人工复核记录",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="review_submit",
            method="POST",
            path="/api/review/submit",
            description="提交带标签的复核结果",
            json_body={
                "requestId": "auto-history-001",
                "decision": "flag",
                "overallProbability": 0.82,
                "overallConfidence": 0.77,
                "label": 1,
                "notes": "automation check",
            },
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
