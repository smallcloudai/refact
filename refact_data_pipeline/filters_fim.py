import random

from refact_encoding import RefactEncoding
from refact_data_pipeline import DatasetOpts

from typing import Dict, Union


class SymbolsMiddleSplit:

    def __init__(self,
                 min_symbols: int = 1,
                 max_symbols: int = 4000,   # 4k is one dense screen of text
        ):
        self._min_symbols = min_symbols
        self._max_symbols = max_symbols

    def split(self, text: str):
        max_symbols = min([
            self._max_symbols,
            len(text) - 1,
        ])
        if self._min_symbols > max_symbols:
            raise RuntimeError
        mid_symbols = random.randint(self._min_symbols, max_symbols)
        assert len(text) - mid_symbols - 1 >= 0
        split_pos = random.randint(0, len(text) - mid_symbols - 1)
        middle = text[split_pos:split_pos + mid_symbols]
        assert len(middle) == mid_symbols

        prefix = text[:split_pos]
        suffix = text[split_pos + mid_symbols:]
        if not suffix and not middle:
            raise RuntimeError

        return prefix, middle, suffix


class FIM:
    def __init__(
        self,
        inner_filter,
        dataopts: DatasetOpts,
    ):
        self.inner_filter = inner_filter
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.fim_probability = dataopts.get("fim_probability", 0.5)
        self.tkr_stochastic_tokens = dataopts.get("tkr_stochastic_tokens", 3)
        self.enc: RefactEncoding = dataopts.encoding
        self.special_tokens = [
            self.enc.PREFIX,
            self.enc.SUFFIX,
            self.enc.INFILL,
            self.enc.EOT,
        ]
        assert len(set(self.special_tokens)) == len(self.special_tokens)
        self.splitter = SymbolsMiddleSplit()

    def __iter__(self):
        stats: Dict[str, Union[int, float]] = {
            "fim_unicode_split": 0,
            "fim_unable_to_split": 0,
            "fim_out": 0,
        }
        for sample in self.inner_filter:
            tokens, _ = self.enc.encode_stochastic(sample["text"], [], 0.01*self.tkr_stochastic_tokens)
            cursor = 0
            while cursor < len(tokens):
                if random.random() > self.fim_probability:
                    # plain text branch
                    plain = tokens[cursor : cursor + self.n_ctx]
                    cursor += len(plain)
                    mask = [1] * len(plain)
                    plain.append(self.enc.EOT)
                    # If last_chunk then the EOT is real, the model should predict it. If not, it just
                    # acts as a separator, the model should not predict it.
                    # And it's not visible anyway if len(plain) > n_ctx
                    last_chunk = sample.get("last_chunk", True)
                    if len(plain) < self.n_ctx and last_chunk:
                        mask.append(1)
                    else:
                        mask.append(0)
                    yield {
                        "tokens": plain,
                        "mask": mask,
                        "first": [1] + [0]*(len(plain) - 1),
                        "stats": {**sample["stats"], **stats},
                    }
                else:
                    # FIM
                    wiggle_low = (self.n_ctx * 9 // 20) if random.randint(0, 2) == 0 else (self.n_ctx * 18 // 20)
                    wiggle = random.randint(wiggle_low, self.n_ctx * 21 // 20)
                    # n_ctx   *9//20  *18//20  *21//20
                    # 4096 -> 2048    3686     4300
                    # 2048 -> 1024    1843     2150
                    pre_fim_toks = tokens[cursor : cursor + wiggle]
                    cursor += len(pre_fim_toks)
                    try:
                        text = self.enc.decode_utf8(pre_fim_toks)
                    except:
                        stats["fim_unicode_split"] += 1
                        continue
                    # To plot distribution:
                    # with open("wiggle.csv", "at") as f:
                    #     f.write(f"{wiggle_low},{wiggle},{len(pre_fim_toks)},{len(text)}\n")
                    try:
                        prefix, middle, suffix = self.splitter.split(text)
                    except (RuntimeError, ValueError):
                        stats["fim_unable_to_split"] += 1
                        continue
                    prefix_toks, _ = self.enc.encode_stochastic(prefix, [], 0.01*self.tkr_stochastic_tokens)
                    suffix_toks, _ = self.enc.encode_stochastic(suffix, [], 0.01*self.tkr_stochastic_tokens)
                    if random.random() < 0.5:
                        tokens_context = [self.enc.PREFIX] + prefix_toks + [self.enc.SUFFIX] + suffix_toks
                        mask_context = [0] + [1] * len(prefix_toks) + [0] + [1] * len(suffix_toks)
                    else:
                        tokens_context = [self.enc.SUFFIX] + suffix_toks + [self.enc.PREFIX] + prefix_toks
                        mask_context = [0] + [1] * len(suffix_toks) + [0] + [1] * len(prefix_toks)
                    middle_toks, _ = self.enc.encode_stochastic(middle, [], 0.01*self.tkr_stochastic_tokens)
                    middle_mask = [1] * len(middle_toks)
                    yield {
                        "tokens": tokens_context + [self.enc.INFILL] + middle_toks + [self.enc.EOT],
                        "mask": mask_context + [0] + middle_mask + [1],
                        "first": [1] + [0]*(-1 + len(tokens_context) + 1 + len(middle_toks) + 1),
                        "stats": {**sample["stats"], **stats},
                    }
                stats["fim_out"] += 1


class CodeExtract:

    def __init__(self, inner_filter, dataopts: DatasetOpts):
        self.inner_filter = inner_filter

    def __iter__(self):
        for sample in self.inner_filter:
            yield {
                "text": sample["code"],
                "stats": sample["stats"],
                "last_chunk": True,
            }

