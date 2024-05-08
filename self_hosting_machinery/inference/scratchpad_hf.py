import torch as th
import time

from typing import List, Any, Dict, Optional, Union, Callable, Set


class ScratchpadHuggingfaceBase:

    def __init__(
        self,
        tokenizer,
        max_tokens: int,
        logger: Callable,
        stop_tokens: Union[str, List[str]],
        created: float,
        **unused
    ):
        self._tokenizer = tokenizer
        self._max_tokens = max_tokens
        self._logger = logger
        self._created = created

        self._stop_lf = False
        self._stop_lf_lf = False
        self._prev_token: Optional[int] = None
        self._stop_tokens: Set[int] = set()
        if isinstance(stop_tokens, str):
            stop_tokens = [stop_tokens]
        for s in stop_tokens:
            if s == "\n":
                self._stop_lf = True
                continue
            if s == "\n\n":
                self._stop_lf_lf = True
                continue
            t = self._tokenizer.encode(s)
            if len(t) == 1:
                self._stop_tokens.add(t[0])
            else:
                self._logger("ScratchpadBase: cannot use '%s' as a stop token" % (s.replace("\n", "\\n")))

        self._tokens_produced = 0
        self._completion = []
        self._eos_token = tokenizer.eos_token_id
        self._special_tokens = {
            *map(self._encode_one_token, filter(lambda x: isinstance(x, str), tokenizer.special_tokens_map.values())),
            *tokenizer.additional_special_tokens_ids
        }

        self.needs_upload = False
        self.finish_reason = ""

        for k, v in unused.items():
            self.debuglog("ScratchpadHuggingfaceBase: unused parameter '%s' = '%s'" % (k, v))

    def after_token_selection(self, m, chosen_token: th.Tensor, **unused) -> Dict[str, Any]:
        t = chosen_token.item()

        if t in [self._tokenizer.eos_token_id]:
            self.finish_reason = "stop-eot"
        elif t in self._special_tokens:
            self.finish_reason = "stop-special-token"

        if not self.finish_reason:
            self._completion.append(t)
        if t in self._stop_tokens:
            self.finish_reason = "stop-token"

        couple_of_tokens_decoded = self._tokenizer.decode(([self._prev_token] if self._prev_token is not None else []) + [t])
        self._prev_token = t
        if self._stop_lf and ("\n" in couple_of_tokens_decoded):
            self.finish_reason = "stop-lf"
        if self._stop_lf_lf and ("\n\n" in couple_of_tokens_decoded):
            self.finish_reason = "stop-lflf"

        self._tokens_produced += 1
        if self._tokens_produced % 5 == 0:
            self.needs_upload = True

        return dict()

    def _encode_one_token(self, text: str) -> int:
        tokens = self._tokenizer.encode(text, add_special_tokens=False)
        if len(tokens) != 1:
            raise ValueError(f"Must be single token, have {tokens} for '{text}'")
        return tokens[0]

    def _encode_without_special_tokens(self, txt: str) -> List[int]:
        if hasattr(self._tokenizer, "tokenizer_copy_but_does_not_encode_special_tokens"):
            t = self._tokenizer.tokenizer_copy_but_does_not_encode_special_tokens
        else:
            t = self._tokenizer.backend_tokenizer
        return t.encode(txt, add_special_tokens=False).ids

    @property
    def generated_tokens_n(self):
        return self._tokens_produced

    @property
    def eos_token(self):
        return self._eos_token

    def prompt(self, T: int):
        raise NotImplementedError()

    def completion(self, final: bool):
        raise NotImplementedError()

    def debuglog(self, *args):
        elapsed = time.time() - self._created
        self._logger("%4.0fms" % (elapsed * 1000,), *args)


class ScratchpadHuggingfaceCompletion(ScratchpadHuggingfaceBase):

    def __init__(self, prompt: str, **kwargs):
        super().__init__(**kwargs)
        self._prompt = prompt

    def prompt(self, T: int):
        return self._tokenizer.encode(self._prompt)

    def completion(self, final: bool):
        return {"text": self._tokenizer.decode(self._completion)}
