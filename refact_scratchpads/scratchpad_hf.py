import torch as th
import time
import termcolor

from refact_scratchpads.scratchpad_utils import trim_context_infill

from typing import List, Any, Dict, Optional, Union, Callable, Set


class EncodingWrapper:

    def __init__(self, tokenizer):
        self._tokenizer = tokenizer

    def encode(self, text: str) -> List[int]:
        return self._tokenizer.encode(text, add_special_tokens=False)

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
        self._tokenizer_skip_first = bool(tokenizer.encode(""))    # XXX: replace with add_special_tokens=False ?
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

        if t in [self._tokenizer.eos_token_id]:
            self.finish_reason = "eot"
        elif t in self._special_tokens:
            self.finish_reason = "special-token"

        if not self.finish_reason:
            self._completion.append(t)
        if t in self._stop_tokens:
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

    def encode_without_special_tokens(self, txt: str) -> List[int]:
        if hasattr(self._tokenizer, "tokenizer_copy_but_does_not_encode_special_tokens"):
            t = self._tokenizer.tokenizer_copy_but_does_not_encode_special_tokens
        else:
            t = self._tokenizer.backend_tokenizer
        return t.encode(txt, add_special_tokens=False).ids

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


class ScratchpadFIM(ScratchpadHuggingfaceBase):

    def __init__(
            self,
            sources: Dict[str, str],
            cursor_file: str,
            cursor0: int,
            cursor1: int,
            **kwargs
    ):
        super().__init__(**kwargs)

        assert cursor0 == cursor1

        self._cursor_file = cursor_file
        self._cursor = cursor0
        self._code = sources[cursor_file]

        self._prefix: Optional[str] = None
        self._suffix: Optional[str] = None
        self._suffix_line0cut: Optional[str] = None
        self._completion = []

        self._tokens_produced = 0
        self._fim_prefix = self._encode_one_token("<fim_prefix>")
        self._fim_suffix = self._encode_one_token("<fim_suffix>")
        self._fim_middle = self._encode_one_token("<fim_middle>")

    def _prompt_format(self, prefix_tokens, suffix_tokens):
        raise NotImplementedError()

    def prompt(self, T: int):
        self._prefix = self._code[:self._cursor]
        # Why we need to cut the line right of the cursor?
        # Example 1:
        # function_call(param1, GENERATED_TONENS<EOF>)
        # => everything works right
        # Example 2:
        # function_call(param1, GENERATED_TONENS)\nMORE_TOKENS\nSOME_OTHER_CALL(OTHER_PARAM<EOF>)
        #                                        ^^ but we stop here because we need single line completion
        # => we have two closing parenthesis.
        # self._suffix = "".join(self._code[self._cursor:].splitlines(keepends=True)[1:])
        self._suffix = self._code[self._cursor:].lstrip(" \t")
        self._suffix_line0cut = "".join(self._code[self._cursor:].splitlines(keepends=True)[1:])
        self._completion.clear()

        prefix_cut, suffix_cut = trim_context_infill(
            self._prefix, self._suffix, EncodingWrapper(self._tokenizer), T - self._max_tokens
        )
        prefix_cut_tokens = self.encode_without_special_tokens(prefix_cut)
        suffix_cut_tokens = self.encode_without_special_tokens(suffix_cut)
        self.debuglog(
            "ScratchpadFIM prompt prefix %d chars -> %d tokens, suffix %d chars -> %d tokens, T=%d max_new_tokens=%d" %
            (len(prefix_cut), len(prefix_cut_tokens), len(suffix_cut), len(suffix_cut_tokens), T, self._max_tokens)
        )
        prompt: List[int] = self._prompt_format(prefix_cut_tokens, suffix_cut_tokens)
        self.debuglog("-"*40)
        self.debuglog(self._tokenizer.decode(prompt))
        self.debuglog("-"*40)
        return prompt

    def completion(self, final: bool):
        assert self._prefix is not None
        assert self._suffix is not None
        completion = self._tokenizer.decode(self._completion)
        if self.finish_reason == "eot":
            # Correct stop
            return {self._cursor_file: self._prefix + completion + self._suffix}
        else:
            # "stop-lf" or "length" or not stopped yet (empty reason), it's better to remove first line remainder
            return {self._cursor_file: self._prefix + completion + self._suffix_line0cut}


class ScratchpadSPM(ScratchpadFIM):

    def _prompt_format(self, prefix_tokens, suffix_tokens):
        return [
            self._fim_suffix,
            *suffix_tokens,
            self._fim_prefix,
            *prefix_tokens,
            self._fim_middle,
        ]


class ScratchpadPSM(ScratchpadFIM):

    def _prompt_format(self, prefix_tokens, suffix_tokens):
        return [
            self._fim_prefix,
            *prefix_tokens,
            self._fim_suffix,
            *suffix_tokens,
            self._fim_middle,
        ]


class ScratchpadCodeLlama(ScratchpadHuggingfaceBase):

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
        self._fim_prefix = self._encode_one_token("<PRE>")
        self._fim_suffix = self._encode_one_token("<SUF>")
        self._fim_middle = self._encode_one_token("<MID>")
        self._fim_eot = self._encode_one_token("<EOT>")
        self._special_tokens.update({
            self._fim_prefix, self._fim_suffix, self._fim_middle, self._fim_eot,
        })

    def prompt(self, T: int):
        self._prefix = self._code[:self._cursor]
        self._suffix = "".join(self._code[self._cursor:].splitlines(keepends=True)[1:])
        self._completion.clear()

        prefix_cut, suffix_cut = trim_context_infill(
            self._prefix, self._suffix, EncodingWrapper(self._tokenizer), T - self._max_tokens)
        prompt: List[int] = [
            self._eos_token,
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
        self._esc = "<empty_output>"

    def _prompt(self) -> str:
        text = self._esc + "SYSTEM You are a programming assistant. If you don't understand the question, just say: I don't understand the question.\n"
        for message in self._messages:
            if message["content"] == "":
                continue
            if message["role"] == "user":
                text += self._esc + "USER " + message["content"].strip() + "\n"
            else:
                text += self._esc + "ASSISTANT " + message["content"].strip() + "\n"
        text += self._esc + "ASSISTANT"
        return text
