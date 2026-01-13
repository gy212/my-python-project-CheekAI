"""后端 /api/history/save 检测历史写入测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

AGGREGATION_SAMPLE = {
    "overallProbability": 0.48,
    "overallConfidence": 0.73,
    "method": "ensemble",
    "thresholds": {"low": 0.3, "medium": 0.5, "high": 0.75, "veryHigh": 0.9},
    "rubricVersion": "v0",
    "decision": "review",
    "bufferMargin": 0.1,
    "stylometryProbability": 0.4,
    "qualityScoreNormalized": 0.2,
    "blockWeights": {"stylometry": 0.4, "perplexity": 0.3},
    "dimensionScores": {"clarity": 4, "fluency": 3},
}

MULTI_ROUND_SAMPLE = {
    "rounds": 2,
    "avgProbability": 0.5,
    "avgConfidence": 0.7,
    "variance": 0.01,
    "details": [
        {"round": 1, "probability": 0.45, "confidence": 0.68},
        {"round": 2, "probability": 0.55, "confidence": 0.72},
    ],
}

scenario = ApiScenario(
    functionality="api_history_save",
    category="backend",
    description="POST /api/history/save 写入一次检测历史",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="history_save",
            method="POST",
            path="/api/history/save",
            description="保存构造的检测结果",
            json_body={
                "id": "auto-history-001",
                "reqParams": {"language": "zh-CN", "providers": []},
                "aggregation": AGGREGATION_SAMPLE,
                "multiRound": MULTI_ROUND_SAMPLE,
            },
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
