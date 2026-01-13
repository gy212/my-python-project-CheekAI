import json
import shutil
import sys
from datetime import datetime
from pathlib import Path
from uuid import uuid4


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    log_file = root / "backend" / "logs" / "glm_last_response.json"
    if not log_file.exists():
        print(f"[skip] log not found: {log_file}")
        return

    try:
        data = json.loads(log_file.read_text(encoding="utf-8", errors="ignore"))
    except Exception as exc:
        print(f"[error] failed to load json: {exc}")
        return

    target_dir = root / "backend" / "logs" / "glm_responses"
    target_dir.mkdir(parents=True, exist_ok=True)

    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    request_id = data.get("id") or data.get("request_id") or str(uuid4())[:8]
    target_file = target_dir / f"{timestamp}_{request_id}.json"

    try:
        shutil.copyfile(log_file, target_file)
        print(f"[ok] saved -> {target_file}")
    except Exception as exc:
        print(f"[error] copy failed: {exc}")
        return


if __name__ == "__main__":
    main()
