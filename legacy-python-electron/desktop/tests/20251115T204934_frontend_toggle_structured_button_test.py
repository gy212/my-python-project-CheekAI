"""前端 结构预览 按钮切换测试脚本，生成时间 2025-11-15T20:49:34+08:00"""

import desktop.tests._path  # noqa: F401
from tests_shared.scenario import ApiScenario, StepConfig
from tests_shared.utils import load_sample_bytes

GENERATED_AT = "2025-11-15T20:49:34+08:00"
SAMPLE_BYTES = load_sample_bytes()


def _files(_ctx):
    return {"file": ("structured.txt", SAMPLE_BYTES, "text/plain")}


def _mark_nodes(ctx, _resp, data):
    if isinstance(data, dict):
        nodes = data.get("structuredNodes") or []
        ctx["structured_count"] = len(nodes)


scenario = ApiScenario(
    functionality="frontend_toggle_structured_button",
    category="frontend",
    description="拉取结构化节点以验证预览可见性切换",
    generated_at=GENERATED_AT,
    steps=[
        StepConfig(
            name="prefetch_structure",
            method="POST",
            path="/api/preprocess/upload",
            description="上传样例文档获取 structuredNodes",
            data_body={
                "autoLanguage": "true",
                "stripHtml": "true",
                "redactPII": "false",
                "normalizePunctuationOpt": "true",
                "chunkSizeTokens": "400",
                "overlapTokens": "80",
            },
            files=_files,
            extract=_mark_nodes,
        ),
    ],
)

if __name__ == "__main__":
    scenario.run()
