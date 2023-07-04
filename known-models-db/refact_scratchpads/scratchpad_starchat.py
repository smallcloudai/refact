import torch as th

from typing import List, Any, Dict

from refact_scratchpads import ScratchpadBase
from refact_encoding import RefactEncoding


class ScratchpadStarChat(ScratchpadBase):

    def __init__(self,
                 enc: RefactEncoding,
                 messages: List[Dict[str, str]],
                 **kwargs):
        super().__init__(enc, **kwargs)

        self._system_message = "\n"
        self._messages = messages

        self._completion = []
        self._tokens_produced = 0

    def before_token_selection(self, m, **unused) -> Dict[str, Any]:
        return dict()

    def after_token_selection(self, m, chosen_token: th.Tensor, **unused) -> Dict[str, Any]:
        t = chosen_token.item()

        if chosen_token in [self.enc.EOT]:
            self.finish_reason = "eot"
        elif chosen_token in [self.enc.END, self.enc.SYSTEM, self.enc.USER, self.enc.ASSISTANT]:
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

        text = _wrap_system_token(self.enc.SYSTEM) + self._system_message + _wrap_system_token(self.enc.END)
        for message in self._messages:
            if message["role"] == "user":
                text += _wrap_system_token(self.enc.SYSTEM)
            else:
                text += _wrap_system_token(self.enc.ASSISTANT)
            text += message["content"] + _wrap_system_token(self.enc.END)
        text += _wrap_system_token(self.enc.ASSISTANT)
        tokens = self.enc.encode(text)
        self.debuglog(f"{len(tokens)} tokens")
        return tokens

    def completion(self, final: bool):
        return {
            "chat__role": "assistant",
            "chat__content": self.enc.decode(self._completion),
        }
