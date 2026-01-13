"""前端 保留当前结果 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

AGGREGATION_SAMPLE = {
    "overallProbability": 0.35,
    "overallConfidence": 0.72,
    "method": "ensemble",
    "thresholds": {"low": 0.3, "medium": 0.5, "high": 0.75, "veryHigh": 0.9},
    "rubricVersion": "v0",
    "decision": "pass",
    "bufferMargin": 0.1,
    "stylometryProbability": 0.33,
    "qualityScoreNormalized": 0.4,
    "blockWeights": {"stylometry": 0.5},
    "dimensionScores": {"clarity": 4},
}

MULTI_ROUND_SAMPLE = {
    "rounds": 2,
    "avgProbability": 0.34,
    "avgConfidence": 0.71,
    "variance": 0.006,
    "details": [
        {"round": 1, "probability": 0.32, "confidence": 0.7},
        {"round": 2, "probability": 0.36, "confidence": 0.72},
    ],
}

scenario = ApiScenario(
    functionality="frontend_keep_result_button",
    category="frontend",
    description="模拟点击保留结果按钮，写入历史并刷新列表",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="history_save",
            method="POST",
            path="/api/history/save",
            description="保存本地缓存的检测结果",
            json_body={
                "id": "ui-keep-btn",
                "reqParams": {"providers": [], "sensitivity": "medium"},
                "aggregation": AGGREGATION_SAMPLE,
                "multiRound": MULTI_ROUND_SAMPLE,
            },
        ),
        StepConfig(
            name="refresh_history",
            method="GET",
            path="/api/history/list",
            description="刷新历史列表界面"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
