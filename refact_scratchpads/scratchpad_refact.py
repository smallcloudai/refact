import termcolor
import torch as th

from refact_encoding import RefactEncoding
from refact_scratchpads.scratchpad import ScratchpadBase
from refact_scratchpads import utils

from typing import List, Any, Dict, Optional


class ScratchpadRefact(ScratchpadBase):
    def __init__(
        self,
        enc: RefactEncoding,
        cursor_file: str,
        cursor0: int,
        cursor1: int,
        function: str,
        sources: Dict[str, str],
        **kwargs
    ):
        super().__init__(enc, **kwargs)

        assert cursor0 == cursor1
        assert function == "infill"

        self._cursor_file = cursor_file
        self._cursor = cursor0
        self._code = sources[cursor_file]

        self._prefix: Optional[str] = None
        self._suffix: Optional[str] = None
        self._completion = []

        self._tokens_produced = 0

    def before_token_selection(self, m, **unused) -> Dict[str, Any]:
        return dict()

    def after_token_selection(
            self,
            m,
            chosen_token: th.Tensor,
            **unused
    ) -> Dict[str, Any]:
        t = chosen_token.item()

        if chosen_token in [self.enc.EOT]:
            self.finish_reason = "eot"
        elif chosen_token in [self.enc.PREFIX, self.enc.SUFFIX, self.enc.INFILL]:
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

        prefix_cut, suffix_cut = utils.trim_context_infill(self._prefix, self._suffix, self.enc, T - self.max_tokens)
        prefix_cut_tokens = self.enc.encode(prefix_cut)
        suffix_cut_tokens = self.enc.encode(suffix_cut)
        prompt: List[int] = [
            self.enc.PREFIX,
            *prefix_cut_tokens,
            self.enc.SUFFIX,
            *suffix_cut_tokens,
            self.enc.INFILL,
        ]
        self._completion.clear()
        return prompt

    def completion(self, final: bool):
        return {
            self._cursor_file: self._prefix + self.enc.decode(self._completion) + self._suffix,
        }
