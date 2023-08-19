import json
from functools import lru_cache
from transformers import AutoTokenizer


@lru_cache(maxsize=100)
def cached_get_tokenizer(name) -> AutoTokenizer:
    tokenizer = AutoTokenizer.from_pretrained(name, trust_remote_code=True)
    j = json.loads(tokenizer.backend_tokenizer.to_str())
    special = []
    for token_dict in j["added_tokens"]:
        content = token_dict["content"]
        special.append(content)
    #     token_dict["content"] = "⬥⬥⬥" + content + "⬥⬥⬥"
    # tokenizer.tokenizer_copy_but_does_not_encode_special_tokens = tokenizer.backend_tokenizer.from_str(
    #     json.dumps(j)
    # )
    tokenizer.special_tokens = special
    return tokenizer
