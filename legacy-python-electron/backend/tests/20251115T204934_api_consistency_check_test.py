"""后端 /api/consistency/check 一致性检测脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

SEGMENT_SAMPLE = [
    {
        "chunkId": 1,
        "language": "zh-CN",
        "offsets": {"start": 0, "end": 12},
        "aiProbability": 0.45,
        "confidence": 0.61,
        "signals": {
            "llmJudgment": {"prob": 0.4, "models": []},
            "perplexity": {"ppl": 25.3, "z": -0.8},
            "stylometry": {"ttr": 0.62, "avgSentenceLen": 21.5},
        },
        "explanations": [
            "ttr 指标提示词汇多样性高",
            "模型同时给出 ttr 词汇多样性低 的反例",
        ],
    }
]

scenario = ApiScenario(
    functionality="api_consistency_check",
    category="backend",
    description="POST /api/consistency/check 解析段落解释冲突",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="consistency_check",
            method="POST",
            path="/api/consistency/check",
            description="提交一段带冲突解释的分段",
            json_body={"segments": SEGMENT_SAMPLE},
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
