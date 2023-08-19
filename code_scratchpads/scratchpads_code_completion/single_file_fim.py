from typing import Any, Callable, Dict, List, Set, Union, Optional, AsyncGenerator, Tuple
from itertools import zip_longest
from code_scratchpads import scratchpad_code_completion
import termcolor


def trim_context_infill(
        prefix: str,
        suffix: str,
        tokenizer: Callable,
        tokens_limit: int
) -> Tuple[str, str]:
    lines_prefix = [(l, 'prefix') for l in reversed(prefix.splitlines(keepends=True))]
    lines_suffix = [(l, 'suffix') for l in suffix.splitlines(keepends=True)]

    merged_lines = [val for pair in zip_longest(lines_prefix, lines_suffix) for val in pair if val]

    lines_prefix_p, lines_suffix_p = [], []
    for line, t in merged_lines:
        if (line_tok_cnt := len(enc.encode(line))) >= tokens_limit: break
        lines_prefix_p.append(line) if t == 'prefix' else lines_suffix_p.append(line)
        tokens_limit -= line_tok_cnt

    prefix = ''.join(reversed(lines_prefix_p))
    suffix = ''.join(lines_suffix_p)
    return prefix, suffix


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
        sampling_parameters_to_patch["stop"] = [self._eot]
        txt: List[str] = self.sources[self.cursor_file]
        prefix_lines = txt[:self.cursor_line] + [txt[self.cursor_line][:self.cursor_character]]
        suffix_lines = [txt[self.cursor_line][self.cursor_character:]] + txt[self.cursor_line + 1:]
        # prefix_reversed = prefix_lines
        # merged_lines = [x for pair in zip_longest(prefix_reversed, suffix_lines) for x in pair if x]
        prefix_result_reversed, suffix_result = [], []
        tokens_limit = context_size - self.max_new_tokens
        for p_line, s_line in zip_longest(prefix_lines[::-1], suffix_lines):
            if p_line:
                if line_tok_cnt := len(self.tokenizer.encode(p_line)) >= tokens_limit:
                    break
                prefix_result_reversed.append(p_line)
                tokens_limit -= line_tok_cnt
            if s_line:
                if line_tok_cnt := len(self.tokenizer.encode(s_line)) >= tokens_limit:
                    break
                suffix_result.append(s_line)
                tokens_limit -= line_tok_cnt
        prefix = '\n'.join(reversed(prefix_result_reversed))
        suffix = '\n'.join(suffix_result)
        for special in self.tokenizer.special_tokens:
            prefix = prefix.replace(special, "")
            suffix = suffix.replace(special, "")
        prompt = self._fim_prefix + prefix + self._fim_suffix + suffix + self._fim_middle
        self._debuglog("SingleFileFIM prompt dump:\n" +
            "-"*80 + "\n" +
            termcolor.colored(self._fim_prefix, 'yellow') +
            termcolor.colored(prefix, 'green') +
            termcolor.colored(self._fim_suffix, 'yellow') +
            termcolor.colored(suffix, 'cyan') +
            termcolor.colored(self._fim_middle, 'yellow') +
            "\n" + "-"*80
        )
        return prompt

    async def re_stream_response(self, text_generator: AsyncGenerator[Any, None]):
        async for model_says in text_generator:
            if "token" in model_says:
                print("token!", model_says["token"]["txt"])
            if "generated_text" in model_says:
                print("generated_text!", model_says["generated_text"])
            yield model_says

        # Why we need to cut the line right of the cursor?
        # Example 1:
        # function_call(param1, GENERATED_TONENS<EOF>)
        # => everything works right
        # Example 2:
        # function_call(param1, GENERATED_TONENS)\nMORE_TOKENS\nSOME_OTHER_CALL(OTHER_PARAM<EOF>)
        #                                        ^^ but we stop here because we need single line completion
        # => we have two closing parenthesis if we stop.
        # self._suffix = self._code[self._cursor:]
        # self._suffix_line0cut = "".join(self._code[self._cursor:].splitlines(keepends=True)[1:])

        # # self.debuglog()

    # def completion(self, final: bool):
    #     assert self._prefix is not None
    #     assert self._suffix is not None
    #     completion = self._tokenizer.decode(self._completion).rstrip(os.linesep)
    #     if self.finish_reason == "eot":
    #         # Correct stop
    #         return {self._cursor_file: self._prefix + completion + self._suffix}
    #     else:
    #         # "stop-lf" or "length" or not stopped yet (empty reason), it's better to remove first line remainder
    #         return {self._cursor_file: self._prefix + completion + self._suffix_line0cut}
