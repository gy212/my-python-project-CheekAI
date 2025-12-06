import os
import json
import threading
import time

_DEFAULT_BASE_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'config')
MAX_VERSIONS = 20

_lock = threading.Lock()


def _base_dir() -> str:
    return os.environ.get('CHEEKAI_CONFIG_DIR') or _DEFAULT_BASE_DIR


def _file_path() -> str:
    return os.path.join(_base_dir(), 'api_config.json')


def _versions_dir() -> str:
    return os.path.join(_base_dir(), 'versions')


def _ensure_dirs():
    os.makedirs(_base_dir(), exist_ok=True)
    os.makedirs(_versions_dir(), exist_ok=True)

def _now_iso():
    return time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime())

def _timestamp():
    return time.strftime('%Y%m%d%H%M%S', time.localtime())

def _read_file(path):
    if not os.path.exists(path):
        return None
    with open(path, 'r', encoding='utf-8') as f:
        try:
            return json.load(f)
        except Exception:
            return None

def _write_atomic(path, data):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False)
    os.replace(tmp, path)


def _rotate_versions():
    versions_dir = _versions_dir()
    files = sorted([p for p in os.listdir(versions_dir)], reverse=True)
    if len(files) > MAX_VERSIONS:
        for p in files[MAX_VERSIONS:]:
            try:
                os.remove(os.path.join(versions_dir, p))
            except Exception:
                pass


class ConfigStore:
    def load(self):
        _ensure_dirs()
        file_path = _file_path()
        data = _read_file(file_path)
        if not data:
            data = {"version": "v0", "updatedAt": _now_iso(), "data": {}}
            _write_atomic(file_path, data)
        return data

    def save(self, cfg):
        _ensure_dirs()
        file_path = _file_path()
        versions_dir = _versions_dir()
        with _lock:
            ts = _timestamp()
            cfg["version"] = ts
            cfg["updatedAt"] = _now_iso()
            _write_atomic(file_path, cfg)
            _write_atomic(os.path.join(versions_dir, ts + '.json'), cfg)
            _rotate_versions()
        return True

    def get(self, key):
        cfg = self.load()
        cur = cfg.get('data', {})
        if not key:
            return cur
        parts = key.split('.')
        for p in parts:
            if isinstance(cur, dict) and p in cur:
                cur = cur[p]
            else:
                return None
        return cur

    def set(self, key, val):
        cfg = self.load()
        cur = cfg.get('data', {})
        parts = key.split('.') if key else []
        if not parts:
            cfg['data'] = val
        else:
            node = cur
            for p in parts[:-1]:
                if p not in node or not isinstance(node[p], dict):
                    node[p] = {}
                node = node[p]
            node[parts[-1]] = val
            cfg['data'] = cur
        self.save(cfg)
        return True

    def delete(self, key):
        cfg = self.load()
        cur = cfg.get('data', {})
        parts = key.split('.')
        node = cur
        for p in parts[:-1]:
            if p not in node or not isinstance(node[p], dict):
                return False
            node = node[p]
        if parts[-1] in node:
            del node[parts[-1]]
            cfg['data'] = cur
            self.save(cfg)
            return True
        return False

    def versions(self):
        _ensure_dirs()
        versions_dir = _versions_dir()
        items = []
        for p in sorted(os.listdir(versions_dir), reverse=True):
            if p.endswith('.json'):
                items.append(p[:-5])
        return items

    def rollback(self, ts):
        versions_dir = _versions_dir()
        path = os.path.join(versions_dir, ts + '.json')
        data = _read_file(path)
        if not data:
            return False
        self.save(data)
        return True


store = ConfigStore()
