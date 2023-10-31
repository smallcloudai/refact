import os
import json
import time
from self_hosting_machinery import env
from typing import Dict, Any, Optional, List


def rcfg_save(path: str, adict: Dict[str, Any]) -> None:
    with open(path + ".tmp", "w") as f:
        json.dump(adict, f, indent=4)
    os.rename(path + ".tmp", path)


def rcfg_load(path: str, default_if_not_found: Optional[Dict[str, Any]]=None) -> Dict[str, Any]:
    if default_if_not_found is not None and not os.path.exists(path):
        return default_if_not_found
    with open(path, "r") as f:
        return json.load(f)


def rcfg_load_not_too_old(path: str, seconds: int, default_if_not_found_or_too_old: Optional[Dict[str, Any]]=None) -> Dict[str, Any]:
    if rcfg_mtime(path) + seconds < time.time():
        return default_if_not_found_or_too_old
    return rcfg_load(path, default_if_not_found_or_too_old)


def rcfg_mtime(path) -> int:
    if not os.path.exists(path):
        return 0
    return os.path.getmtime(path)


def rcfg_list_uploads(uploaded_path: str) -> List[str]:
    return list(sorted(os.listdir(uploaded_path)))
