import json
import os

from enum import Enum
from typing import Optional

from huggingface_hub import repo_info
from huggingface_hub.utils import GatedRepoError
from huggingface_hub.utils import RepositoryNotFoundError
from refact_utils.scripts import env


def is_hf_hub_offline() -> bool:
    # NOTE: don't check HF_HUB_OFFLINE env variable here
    return os.path.exists(env.FLAG_HF_HUB_OFFLINE)


def huggingface_hub_token() -> Optional[str]:
    try:
        with open(env.CONFIG_INTEGRATIONS, "r") as f:
            return json.load(f)["huggingface_api_key"]
    except:
        return None


class RepoStatus(Enum):
    OPEN = "open"
    GATED = "gated"
    NOT_FOUND = "not_found"
    UNKNOWN = "unknown"


def has_repo_access(repo_id: str) -> bool:
    if is_hf_hub_offline():
        return False
    try:
        token = huggingface_hub_token()
        repo_info(repo_id=repo_id, token=token)
        return True
    except:
        return False


def get_repo_status(repo_id: str) -> RepoStatus:
    try:
        if is_hf_hub_offline():
            raise RuntimeError("hf is offline")
        token = huggingface_hub_token()
        info = repo_info(repo_id=repo_id, token=token)
        if isinstance(info.gated, str):
            return RepoStatus.GATED
        return RepoStatus.OPEN
    except GatedRepoError:
        return RepoStatus.GATED
    except RepositoryNotFoundError:
        return RepoStatus.NOT_FOUND
    except:
        return RepoStatus.UNKNOWN


if __name__ == "__main__":
    print(get_repo_status("mistralai/Mixtral-8x7B-Instruct-v0.01"))
