"""后端 /api/config/glm 保存密钥测试脚本（生成时间: 2025-11-15T20:49:34+08:00）"""

import backend.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="config_glm_post",
    category="backend",
    description="POST /api/config/glm 保障密钥写入",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="save_key",
            method="POST",
            path="/api/config/glm",
            description="提交测试密钥",
            json_body={"apiKey": "sk-test-automation"},
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
