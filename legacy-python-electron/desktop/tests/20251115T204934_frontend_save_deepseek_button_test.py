"""前端 保存glm密钥 按钮测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig

GENERATED_AT = "2025-11-15T20:49:34+08:00"

scenario = ApiScenario(
    functionality="frontend_save_glm_button",
    category="frontend",
    description="模拟点击保存密钥按钮，检查密钥保存与模型刷新",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="upload_key",
            method="POST",
            path="/api/config/glm",
            description="保存用户输入的 glm API Key",
            json_body={"apiKey": "sk-front-test"},
        ),
        StepConfig(
            name="refresh_providers",
            method="GET",
            path="/api/providers",
            description="刷新模型列表确保含 glm"
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
