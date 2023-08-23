from typing import Any, Dict, List, Optional, AsyncGenerator
from itertools import zip_longest
from code_scratchpads import scratchpad_code_completion
import termcolor


class SingleFileFIM(scratchpad_code_completion.ScratchpadCodeCompletion):
    def __init__(
        self,
        **kwargs,
    ):
        super().__init__(**kwargs)
        def _assert_one_token(text: str):
            tokens = self.tokenizer.encode(text)
            assert len(tokens) == 1
            return text
        self._fim_prefix = _assert_one_token("<fim_prefix>")
        self._fim_suffix = _assert_one_token("<fim_suffix>")
        self._fim_middle = _assert_one_token("<fim_middle>")
        self._eot = _assert_one_token("<|endoftext|>")
        self._prefix: Optional[str] = None
        self._suffix: Optional[str] = None
        self._suffix_line0cut: Optional[str] = None

    def prompt(self, context_size: int, sampling_parameters_to_patch: Dict[str, Any]):
        if self.supports_stop:
            stop = [self._eot, "\n\n"]
            if not self.multiline:
                stop.append("\n")
            sampling_parameters_to_patch["stop"] = stop
        txt: List[str] = self.sources[self.cursor_file]
        if self.cursor_line >= len(txt):
            prefix_lines = txt
            suffix_lines = []
        else:
            prefix_lines = txt[:self.cursor_line] + [txt[self.cursor_line][:self.cursor_character]]
            suffix_lines = txt[self.cursor_line + 1:]
            if self.multiline:
                suffix_lines.insert(0, txt[self.cursor_line][self.cursor_character:])
        prefix_result_reversed, suffix_result = [], []
        tokens_limit = context_size - self.max_new_tokens
        for p_line, s_line in zip_longest(prefix_lines[::-1], suffix_lines):
            if p_line is not None:
                if (line_tok_cnt := len(self.tokenizer.encode(p_line))) >= tokens_limit:
                    break
                prefix_result_reversed.append(p_line)
                tokens_limit -= line_tok_cnt
            if s_line is not None:
                if (line_tok_cnt := len(self.tokenizer.encode(s_line))) >= tokens_limit:
                    break
                suffix_result.append(s_line)
                tokens_limit -= line_tok_cnt
        prefix = '\n'.join(reversed(prefix_result_reversed))
        suffix = '\n'.join(suffix_result)
        for special in self.tokenizer.special_tokens:
            prefix = prefix.replace(special, "")
            suffix = suffix.replace(special, "")
        prompt = self._fim_prefix + prefix + self._fim_suffix + suffix + self._fim_middle
        self._debuglog("SingleFileFIM prompt dump multiline=%i:\n" % self.multiline +
            termcolor.colored(self._fim_prefix, 'yellow') +
            termcolor.colored(prefix, 'green') +
            termcolor.colored(self._fim_suffix, 'yellow') +
            termcolor.colored(suffix, 'cyan') +
            termcolor.colored(self._fim_middle, 'yellow')
        )
        return prompt

    async def re_stream_response(self, text_generator: AsyncGenerator[Any, None]):
        async for model_says in text_generator:
            if "token" in model_says:
                # Streaming tokens one by one
                t = model_says["token"]["text"]
                if t == self._eot:
                    return
                if "\n\n" in t or ("\n" in t and not self.multiline):
                    yield {"code_completion_delta": self.cut_result(t)}
                    return
                yield {"code_completion_delta": t}
            if isinstance(model_says, list):
                ans = [{"code_completion": self.cut_result(x["generated_text"])} for x in model_says]
                if len(ans) >= 1:
                    self._debuglog("SingleFileFIM completion: \"%s\"" % ans[0]["code_completion"].replace("\n", "\\n"))
                yield ans

    def cut_result(self, txt: str):
        cut_at = [
            txt.find(self._eot),
            txt.find("\n\n"),
        ]
        if not self.multiline:
            cut_at.append(txt.find("\n"))
        cut_at = [x for x in cut_at if x != -1]
        if cut_at:
            return txt[:min(cut_at)]
        return txt
