import aiohttp
import aiofiles

from pathlib import Path
from typing import Optional, List, Dict

from refact_utils.scripts import env


__all__ = [
    "load_tokenizer",
    "get_tokenizers",
    "upload_tokenizer",
    "delete_tokenizer",
]


async def _passthrough_tokenizer(uri: str) -> str:
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(uri) as resp:
                return await resp.text()
    except Exception as e:
        raise RuntimeError(f"Failed to download tokenizer from '{uri}': {str(e)}")


async def load_tokenizer(tokenizer_id: Optional[str]) -> str:
    if tokenizer_id is not None:
        tokenizer_path = Path(env.DIR_TOKENIZERS) / f"{tokenizer_id}.json"
        if tokenizer_path.exists():
            try:
                async with aiofiles.open(tokenizer_path, mode='r') as f:
                    return await f.read()
            except Exception as e:
                raise RuntimeError(f"Failed to read tokenizer file '{tokenizer_path}': {str(e)}")
        else:
            raise RuntimeError(f"Tokenizer '{tokenizer_id}' not found")
    else:
        model_path = "Xenova/gpt-4o"
        tokenizer_url = f"https://huggingface.co/{model_path}/resolve/main/tokenizer.json"
        return await _passthrough_tokenizer(tokenizer_url)


def _tokenizers_dir() -> Path:
    return Path(env.DIR_TOKENIZERS)


def _tokenizer_file_to_id(filename: Path) -> str:
    if not filename.exists():
        raise RuntimeError(f"filename not exists `{filename}`")
    if not filename.is_relative_to(_tokenizers_dir()):
        raise RuntimeError(f"filename is not in tokenizers dir `{filename}`")
    if not filename.name.endswith(".json"):
        raise RuntimeError(f"invalid tokenizer filename `{filename.name}`")
    return ".".join(filename.name.split(".")[:-1])


def _tokenizer_id_to_file(tokenizer_id: str) -> Path:
    return _tokenizers_dir() / f"{tokenizer_id}.json"


def get_tokenizers() -> Dict[str, List[str]]:
    default_tokenizers = []
    custom_tokenizers = []
    for filename in sorted(_tokenizers_dir().iterdir()):
        try:
            custom_tokenizers.append(_tokenizer_file_to_id(filename))
        except Exception:
            pass
    return {
        "default": default_tokenizers,
        "custom": custom_tokenizers,
    }


async def upload_tokenizer(tokenizer_id: str, file):
    if not _tokenizers_dir().exists():
        raise RuntimeError(f"no tokenizers dir `{_tokenizers_dir()}`")

    filename = _tokenizer_id_to_file(tokenizer_id)
    if filename.exists():
        raise RuntimeError(f"can't upload tokenizer with id `{tokenizer_id}`, already exists")

    tmp_filename = Path(f"{filename}.tmp")
    tmp_filename.unlink(missing_ok=True)
    try:
        with open(tmp_filename, "wb") as f:
            while True:
                if not (contents := await file.read(1024 * 1024)):
                    break
                f.write(contents)
        tmp_filename.rename(filename)
    except Exception as e:
        filename.unlink(missing_ok=True)
        tmp_filename.unlink(missing_ok=True)
        raise RuntimeError(f"can't upload tokenizer with id `{tokenizer_id}`: {e}")


def delete_tokenizer(tokenizer_id: str):
    filename = _tokenizer_id_to_file(tokenizer_id)
    filename.unlink(missing_ok=True)
