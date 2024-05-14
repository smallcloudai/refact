import json

from typing import Optional

from huggingface_hub import repo_info
from huggingface_hub.utils import GatedRepoError
from huggingface_hub.utils import RepositoryNotFoundError
from refact_utils.scripts import env


def huggingface_hub_token() -> Optional[str]:
    try:
        with open(env.CONFIG_INTEGRATIONS, "r") as f:
            return json.load(f)["huggingface_api_key"]
    except:
        return None


def has_access_to_repo(repo_id: str) -> bool:
    try:
        token = huggingface_hub_token()
        repo_info(repo_id=repo_id, token=token)
        return True
    except GatedRepoError:
        # NOTE: user has no access to this repo
        return False
    except RepositoryNotFoundError:
        # NOTE: repo does not exist, probably bug in our code
        return False
    except:
        return False


if __name__ == "__main__":
    print(has_access_to_repo("mistralai/Mixtral-8x22B-Instruct-v0.1000"))
