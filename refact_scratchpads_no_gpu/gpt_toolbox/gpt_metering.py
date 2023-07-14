from typing import Tuple

import functools
import tiktoken


def gpt_prices(  # Apr 4 2023:
        model_name: str,
) -> Tuple[int, int]:
    # GPT-4 8K prompt[$0.03 / 1K tokens] generated[$0.06 / 1K tokens]
    if model_name.startswith("gpt-4") or model_name.startswith("gpt4"):
        pp1000t_prompt = 30_000
        pp1000t_generated = 60_000
    # gpt-3.5-turbo $0.002 / 1K tokens
    elif model_name.startswith("gpt-3.5-turbo"):
        pp1000t_prompt = 2_000
        pp1000t_generated = 2_000
    else:
        raise ValueError(f'get_prices: Unknown model: {model_name}')
    return pp1000t_prompt, pp1000t_generated


@functools.lru_cache(maxsize=10)
def engine_to_encoding(engine: str) -> tiktoken.Encoding:
    enc = tiktoken.encoding_for_model(engine)
    return enc


engine_to_encoding("text-davinci-003")  # this immediately tests if tiktoken works or not


def calculate_chat_tokens(model_name, messages, completion):
    enc = engine_to_encoding(model_name)
    calc_prompt_tokens_n = 2  # warmup
    for d in messages:
        calc_prompt_tokens_n += len(enc.encode(d["content"], disallowed_special=()))
        calc_prompt_tokens_n += len(enc.encode(d["role"], disallowed_special=()))
        calc_prompt_tokens_n += 4  # to switch user/assistant
    calc_generated_tokens_n = len(enc.encode(completion, disallowed_special=())) + 2  # one to switch, another EOF
    return calc_prompt_tokens_n, calc_generated_tokens_n
