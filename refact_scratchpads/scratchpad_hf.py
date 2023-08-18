import torch as th
import time
import termcolor

from refact_scratchpads.scratchpad_utils import trim_context_infill

from typing import List, Any, Dict, Optional, Union, Callable, Set


class EncodingWrapper:

    def __init__(self, tokenizer):
        self._tokenizer = tokenizer

    def encode(self, text: str) -> List[int]:
        return self._tokenizer.encode(text)

    def decode(self, tokens: List[int]) -> str:
        return self._tokenizer.decode(tokens)


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
        self._tokenizer_skip_first = bool(tokenizer.encode(""))
        self._max_tokens = max_tokens
        self._logger = logger
        self._created = created

        self._stop_lf = False
        self._stop_lf_lf = False
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
        self.debuglog("%05d %s" % (t, self._tokenizer.decode([t]).replace("\n", "\\n")))

        if chosen_token in [self._tokenizer.eos_token_id]:
            self.finish_reason = "eot"
        elif chosen_token in self._special_tokens:
            self.finish_reason = "special-token"

        if not self.finish_reason:
            self._completion.append(t)
        if chosen_token in self._stop_tokens:
            self.finish_reason = "stoptoken"

        t_str = self._tokenizer.decode([t])
        if self._stop_lf and t_str.startswith("\n"):
            self.finish_reason = "stop-lf"
        if self._stop_lf_lf and t_str.startswith("\n\n"):
            self.finish_reason = "stop-lflf"

        self._tokens_produced += 1
        if self._tokens_produced % 5 == 0:
            self.needs_upload = True

        return dict()

    def _encode_one_token(self, text: str) -> int:
        tokens = self._tokenizer.encode(text)
        if self._tokenizer_skip_first:
            tokens = tokens[1:]
        if len(tokens) != 1:
            raise ValueError(f"Must be single token, have {tokens} for '{text}'")
        return tokens[0]

    @property
    def generated_tokens_n(self):
        return self._tokens_produced

    def prompt(self, T: int):
        raise NotImplementedError()

    def completion(self, final: bool):
        raise NotImplementedError()

    def toplevel_fields(self):
        return {}

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


class ScratchpadHuggingface(ScratchpadHuggingfaceBase):
    def __init__(self, sources: Dict[str, str], cursor_file: str, cursor0: int, cursor1: int, **kwargs):
        super().__init__(**kwargs)

        assert cursor0 == cursor1

        self._cursor_file = cursor_file
        self._cursor = cursor0
        self._code = sources[cursor_file]

        self._prefix: Optional[str] = None
        self._suffix: Optional[str] = None
        self._completion = []

        self._tokens_produced = 0
        self._fim_prefix = self._encode_one_token("<fim_prefix>")
        self._fim_suffix = self._encode_one_token("<fim_suffix>")
        self._fim_middle = self._encode_one_token("<fim_middle>")

    def prompt(self, T: int):
        self._prefix = self._code[:self._cursor]
        self._suffix = "".join(self._code[self._cursor:].splitlines(keepends=True)[1:])
        self._completion.clear()

        prefix_cut, suffix_cut = trim_context_infill(
            self._prefix, self._suffix, EncodingWrapper(self._tokenizer), T - self._max_tokens)
        prompt: List[int] = [
            self._fim_prefix,
            *self._tokenizer.encode(prefix_cut),
            self._fim_suffix,
            *self._tokenizer.encode(suffix_cut),
            self._fim_middle,
        ]
        return prompt

    def completion(self, final: bool):
        assert self._prefix is not None
        assert self._suffix is not None
        return {
            self._cursor_file: self._prefix + self._tokenizer.decode(self._completion) + self._suffix,
        }


class ScratchpadChatBase(ScratchpadHuggingfaceBase):

    def __init__(self, messages: List[Dict[str, str]], **kwargs):
        super().__init__(**kwargs)

        self._messages = messages

    def _prompt(self) -> str:
        raise NotImplementedError()

    def prompt(self, T: int):
        self._completion = []
        text = self._prompt()
        tokens = self._tokenizer.encode(text)
        self.debuglog(termcolor.colored(str(self._messages), "yellow"))
        self.debuglog(termcolor.colored(text, "red"))
        self.debuglog(f"prompt {len(tokens)} tokens")
        return tokens

    def completion(self, final: bool):
        return {
            "chat__role": "assistant",
            "chat__content": self._tokenizer.decode(self._completion),
        }


class ScratchpadHuggingfaceStarChat(ScratchpadChatBase):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self._chat_end = self._encode_one_token("<|end|>")
        self._chat_system = self._encode_one_token("<|system|>")
        self._chat_assistant = self._encode_one_token("<|assistant|>")
        self._chat_user = self._encode_one_token("<|user|>")

    def _prompt(self) -> str:
        def _wrap_system_token(t: int) -> str:
            return self._tokenizer.decode([t]) + "\n"

        text = _wrap_system_token(self._chat_system) + "\n" + _wrap_system_token(self._chat_end)
        for message in self._messages:
            if message["content"] == "":
                continue
            if message["role"] == "user":
                text += _wrap_system_token(self._chat_user)
            else:
                text += _wrap_system_token(self._chat_assistant)
            text += message["content"] + _wrap_system_token(self._chat_end)
        text += _wrap_system_token(self._chat_assistant)
        return text


class ScratchpadHuggingfaceWizard(ScratchpadChatBase):

    def _prompt(self) -> str:
        text = ""
        for message in self._messages:
            if message["content"] == "":
                continue
            if message["role"] == "user":
                text += "USER: "
            else:
                text += "ASSISTANT: "
            text += message["content"].strip() + "\n\n"
        text += "ASSISTANT:"
        return text


class ScratchpadHuggingfaceLlama2(ScratchpadChatBase):

    def _prompt(self) -> str:
        text = "<<SYS>>\n" \
               "You are a helpful, respectful and honest assistant. Always answer as helpfully as possible, " \
               "while being safe.  Your answers should not include any harmful, unethical, racist, sexist, " \
               "toxic, dangerous, or illegal content. Please ensure that your responses are socially unbiased " \
               "and positive in nature. If a question does not make any sense, or is not factually coherent, " \
               "explain why instead of answering something not correct. If you don't know the answer to a " \
               "question, please don't share false information.\n" \
               "<</SYS>>\n"
        for message in self._messages:
            if message["content"] == "":
                continue
            if message["role"] == "user":
                text += f"[INST]: {message['content']}[/INST]"
            else:
                text += message["content"] + "\n"
        return text


class ScratchpadHuggingfaceRefact(ScratchpadChatBase):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._esc_token = self._encode_one_token("<empty_output>")

    def _prompt(self) -> str:
        esc = self._tokenizer.decode(self._esc_token)
        system_prompt = "You are a chat bot"
        text = f"{esc}SYSTEM {system_prompt}\n"
        for message in self._messages:
            if message["content"] == "":
                continue
            if message["role"] == "user":
                text += f"{esc}USER "
            else:
                text += f"{esc}ASSISTANT "
            text += message["content"] + "\n"
        text += f"{esc}ASSISTANT "
        return text
