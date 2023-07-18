import torch as th

from refact_scratchpads.scratchpad import ScratchpadBase
from refact_scratchpads import utils

from typing import List, Any, Dict, Optional


class EncodingHuggingface:

    def __init__(self, tokenizer):
        self._tokenizer = tokenizer

    def encode(self, text: str) -> List[int]:
        return self._tokenizer.encode(text)

    def decode(self, tokens: List[int]) -> str:
        return self._tokenizer.decode(tokens)


# TODO: we need to revise ScratchpadBase ASAP
class ScratchpadHuggingface(ScratchpadBase):
    def __init__(self, tokenizer, sources: Dict[str, str],
                 cursor_file: str, cursor0: int, cursor1: int,
                 **kwargs):
        super().__init__(EncodingHuggingface(tokenizer), **kwargs)

        assert cursor0 == cursor1

        self._cursor_file = cursor_file
        self._cursor = cursor0
        self._code = sources[cursor_file]

        self._prefix: Optional[str] = None
        self._suffix: Optional[str] = None
        self._completion = []

        self._tokens_produced = 0
        self._endoftext = self._encode_one_token("<|endoftext|>")
        self._fim_prefix = self._encode_one_token("<fim_prefix>")
        self._fim_suffix = self._encode_one_token("<fim_suffix>")
        self._fim_middle = self._encode_one_token("<fim_middle>")

    def _encode_one_token(self, text: str) -> int:
        tokens = self.enc.encode(text)
        if len(tokens) != 1:
            raise ValueError(f"Too many tokens {tokens} for '{text}'")
        return tokens[0]

    def before_token_selection(self, m, **unused) -> Dict[str, Any]:
        return dict()

    def after_token_selection(
            self,
            m,
            chosen_token: th.Tensor,
            **unused
    ) -> Dict[str, Any]:
        t = chosen_token.item()

        if chosen_token in [self._endoftext]:
            self.finish_reason = "eot"
        elif chosen_token in [self._fim_prefix, self._fim_suffix, self._fim_middle]:
            self.finish_reason = "special-token"

        if not self.finish_reason:
            self._completion.append(t)
        if chosen_token in self.stop_tokens:
            self.finish_reason = "stoptoken"

        t_str = self.enc.decode([t])
        if self.stop_lf and t_str.startswith("\n"):
            self.finish_reason = "stop-lf"
        if self.stop_lf_lf and t_str.startswith("\n\n"):
            self.finish_reason = "stop-lflf"

        self._tokens_produced += 1
        if self._tokens_produced % 5 == 0:
            self.needs_upload = True

        return dict()

    def prompt(self, T: int):
        self._prefix = self._code[:self._cursor]
        self._suffix = "".join(self._code[self._cursor:].splitlines(keepends=True)[1:])
        self._completion.clear()

        prefix_cut, suffix_cut = utils.trim_context_infill(self._prefix, self._suffix, self.enc, T - self.max_tokens)
        prompt: List[int] = [
            self._fim_prefix,
            *self.enc.encode(prefix_cut),
            self._fim_suffix,
            *self.enc.encode(suffix_cut),
            self._fim_middle,
        ]
        return prompt

    def completion(self, final: bool):
        assert self._prefix is not None
        assert self._suffix is not None
        return {
            self._cursor_file: self._prefix + self.enc.decode(self._completion) + self._suffix,
        }


class ScratchpadChatBase(ScratchpadBase):

    def __init__(self, tokenizer, messages: List[Dict[str, str]], **kwargs):
        super().__init__(EncodingHuggingface(tokenizer), **kwargs)

        # self._system_message = "\n"
        self._messages = messages

        self._completion = []
        self._tokens_produced = 0

        self._endoftext = self._encode_one_token("<|endoftext|>")
        self._chat_end = self._encode_one_token("<|end|>")
        self._chat_system = self._encode_one_token("<|system|>")
        self._chat_assistant = self._encode_one_token("<|assistant|>")
        self._chat_user = self._encode_one_token("<|user|>")

    @property
    def _system_message(self):
        raise NotImplementedError()

    def _encode_one_token(self, text: str) -> int:
        tokens = self.enc.encode(text)
        if len(tokens) != 1:
            raise ValueError(f"Too many tokens {tokens} for '{text}'")
        return tokens[0]

    def before_token_selection(self, m, **unused) -> Dict[str, Any]:
        return dict()

    def after_token_selection(self, m, chosen_token: th.Tensor, **unused) -> Dict[str, Any]:
        t = chosen_token.item()

        if chosen_token in [self._endoftext]:
            self.finish_reason = "eot"
        elif chosen_token in [self._chat_end, self._chat_system, self._chat_user, self._chat_assistant]:
            self.finish_reason = "chat-stop-seq"

        if not self.finish_reason:
            self._completion.append(t)
        if chosen_token in self.stop_tokens:
            self.finish_reason = "stoptoken"

        self._tokens_produced += 1
        if self._tokens_produced % 3 == 0:
            self.needs_upload = True

        return dict()

    def prompt(self, T: int):
        self._completion = []

        def _wrap_system_token(t: int) -> str:
            return self.enc.decode([t]) + "\n"

        text = _wrap_system_token(self._chat_system) + self._system_message + _wrap_system_token(self._chat_end)
        for message in self._messages:
            if message["role"] == "user":
                text += _wrap_system_token(self._chat_user)
            else:
                text += _wrap_system_token(self._chat_assistant)
            text += message["content"] + _wrap_system_token(self._chat_end)
        text += _wrap_system_token(self._chat_assistant)
        tokens = self.enc.encode(text)
        self.debuglog(f"{len(tokens)} tokens")
        return tokens

    def completion(self, final: bool):
        return {
            "chat__role": "assistant",
            "chat__content": self.enc.decode(self._completion),
        }


class ScratchpadHuggingfaceStarChat(ScratchpadChatBase):

    @property
    def _system_message(self):
        raise NotImplementedError()
