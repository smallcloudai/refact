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
    tokenizer.special_tokens = special
    # slash_n = []
    # slash_n_slash_n = []
    # for txt, idx in j["model"]["vocab"].items():
    #     if "ĊĊ" in txt:
    #         slash_n_slash_n.append(tokenizer.decode([idx]))
    #     if "Ċ" in txt:
    #         slash_n.append(tokenizer.decode([idx]))
    # tokenizer.slash_n = slash_n
    # tokenizer.slash_n_slash_n = slash_n_slash_n
    return tokenizer


if __name__ == "__main__":
    cached_get_tokenizer("bigcode/starcoder")
