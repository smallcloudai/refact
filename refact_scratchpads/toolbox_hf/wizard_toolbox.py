from typing import Any, Dict

from refact_scratchpads.scratchpad_hf import ScratchpadHuggingfaceBase


def prompt_add_console_logs(selection: str):
    def preprocess(string):
        return "\n".join(string.splitlines()[1:-1])

    example1 = {
        'selection': """
    line = 'hello'
    line += ', world!'
        """,
        'result': """
    line = 'hello'
    print(f'line: {line}')
    line += ', world!'
    print(f'line: {line}')
        """
    }
    print('example:\n', preprocess(example1["result"]))
    print(f'selection:\n{selection}')
    prompt = f'USER: hi, assistant! I need you to add console logs for each line of the code given!\n' \
             f'ASSISTANT: hi, human! sure thing. Please provide me the code!\n' \
             f'USER: {preprocess(example1["selection"])}\n' \
             f'ASSISTANT: {preprocess(example1["result"])}<eot>\n' \
             f'USER: thank you so much, assistant!\n' \
             f'ASSISTANT: no problem, human! feel free to provide me please another piece of code!\n' \
             f'USER: {selection}\n' \
             f'ASSISTANT: '
    return prompt


class ScratchpadHuggingfaceWizardFunctions(ScratchpadHuggingfaceBase):
    def __init__(
            self,
            tokenizer: Any,
            cursor_file: str,
            cursor0: int,
            cursor1: int,
            function: str,
            sources: Dict[str, str],
            **kwargs
    ):
        super().__init__(
            tokenizer=tokenizer,
            stop_tokens='<eot>',  # TODO: stop_token does not work
            **{k: v for k, v in kwargs.items() if k != 'stop_tokens'}
        )
        self._tokenizer: Any = tokenizer
        self._cursor_file: str = cursor_file
        self._cursor0: int = cursor0
        self._cursor1: int = cursor1
        self._function: str = function
        self._sources: Dict[str, str] = sources

        self.prefix: str = ""
        self.suffix: str = ""
        self.selection: str = ""
        self._tokens_produced: int = 0

    def _split_source_prefix_suffix_selection(self, only_full_lines: bool = True):
        from refact_scratchpads.scratchpad_utils import full_line_selection
        source = ""
        for fn, text in self._sources.items():
            if fn == self._cursor_file:
                source = text
                break
        lines = source.splitlines()
        if len(lines) == 0:
            lines.append("\n")
        if lines[-1] == "" or lines[-1][-1] != "\n":
            lines[-1] += "\n"
        join_back = "\n".join(lines)
        if only_full_lines:
            self.cursor0, self.cursor1, self.selection = full_line_selection(self._cursor0, self._cursor1, join_back)
        else:
            self.selection = ""
        self.prefix = join_back[:self.cursor0]
        self.suffix = join_back[self.cursor1:]

    def fun_add_console_logs(self):
        self._split_source_prefix_suffix_selection()
        return self._tokenizer.encode(prompt_add_console_logs(self.selection))

    def prompt(self, T: int):
        if self._function.startswith("add-console-logs"):
            return self.fun_add_console_logs()
        else:
            raise NotImplementedError(f'function {self._function} is not implemented!')

    def completion(self, final: bool) -> Dict[str, str]:
        completion_text = self._tokenizer.decode(self._completion)
        result = {}
        result[self._cursor_file] = self.prefix + completion_text + self.suffix
        return result
