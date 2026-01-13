"""后端 /api/paper/analyze 批阅增强测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="api_paper_analyze",
    category="backend",
    description="POST /api/paper/analyze 触发多轮可读性分析",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="paper_analyze",
            method="POST",
            path="/api/paper/analyze",
            description="提交样例文本以获取可读性分析",
            json_body={
                "text": "CheekAI 自动化测试正在验证论文分析接口的稳定性。这段文本包含多句描述，以便触发多轮打分。",
                "language": "zh-CN",
                "genre": "essay",
                "rounds": 3,
                "useLLM": False,
            },
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
