import aiohttp
import aiofiles

from pathlib import Path
from typing import Optional
from refact_utils.scripts import env


async def _passthrough_tokenizer(uri: str) -> str:
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(uri) as resp:
                return await resp.text()
    except Exception as e:
        raise RuntimeError(f"Failed to download tokenizer from '{uri}': {str(e)}")


async def get_tokenizer(tokenizer_id: Optional[str]) -> str:
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
