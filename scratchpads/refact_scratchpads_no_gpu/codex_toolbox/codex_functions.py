from .codex_scratchpad import ScratchpadCodex


class ScratchpadCompleteSelectedCode(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='code-davinci-002',
            sp_function_name='complete-selected-code',
        )

    def _prompt(self) -> str:
        prompt = self._txt[:self.cursor0] + self._selection
        return prompt

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor0] + self._selection + completion + self._txt[self.cursor1:]


class ScratchpadFixBug(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='fix-bug',
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}\n', '//') +
            # self._txt[:self.cursor0] +
            self._language_file.replace_comment_line('// --- FIX BUGS ---', '//') + '\n' +
            self._selection +
            self._language_file.replace_comment_line('\n// --- /FIX BUGS ---', '//')
            # + self._txt[self.cursor1:]
        )
        self.stop_sequences = self._language_file.replace_comment_line('\n// --- /BUGFIX ---', '//')
        return prompt

    def _postprocess(self, completion: str) -> str:
        if completion.startswith('\n'):
            completion = completion[1:]

        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


class ScratchpadExplainError(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='explain-error',
            stop_sequences=['\n"""', '\n\"\"\"']
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._selection
        )
        return prompt

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor0] + self._selection + completion + self._txt[self.cursor1:]


class ScratchpadAddConsoleLogs(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='add-console-logs',
            stop_sequences=['\n"""', '\n\"\"\"']
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}', '//') + '\n' +
            self._selection
        )
        return prompt

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


# class ScratchpadExplainCodeBlock(ScratchpadCodex):
#     def __init__(self, **kwargs):
#         super().__init__(
#             **kwargs,
#             engine='code-davinci-002',
#             sp_function_name='explain-code-block',
#         )
#
#     def _prompt(self) -> str:
#         prompt = self._pe_file.replace(
#             '{___place_code_here___}',
#             self._language_file.replace_comment_line(f'// language: {self._language_file.language}', '//') + '\n' +
#             self._txt[:self.cursor0] +
#             self._language_file.replace_comment_line('// --- EXPLAIN CODE ---', '//') + '\n' +
#             self._selection +
#             self._language_file.replace_comment_line('\n// --- /EXPLAIN CODE ---', '//') +
#             self._txt[self.cursor1:]
#         )
#         self.stop_sequences = self._language_file.replace_comment_line('\n// --- /EXPLAINED ---', '//')
#         return prompt
#
#     def _postprocess(self, completion: str) -> str:
#         if completion.startswith('\n'):
#             completion = completion[1:]
#
#         return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


class ScratchpadExplainCodeBlock(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='explain-code-block',
            stop_sequences=['\n"""', '\n\"\"\"']
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}', '//') + '\n' +
            self._selection
        )
        return prompt

    def _postprocess(self, completion: str) -> str:
        # if completion.startswith('\n'):
        #     completion = completion[1:]

        return self._txt[:self.cursor0] + self._selection + completion + self._txt[self.cursor1:]


class ScratchpadCommentEachLine(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='comment-each-line',
            stop_sequences=['\n"""', '\n\"\"\"']
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}', '//') + '\n' +
            self._selection
        )
        return prompt

    def _postprocess(self, completion: str) -> str:
        if completion.startswith('\n'):
            completion = completion[1:]

        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


class ScratchpadMakeCodeShorter(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='code-davinci-002',
            sp_function_name='make-code-shorter',
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}\n', '//') +
            self._txt[:self.cursor0] +
            self._language_file.replace_comment_line('// --- SIMPLIFY THIS ---', '//') + '\n' +
            self._selection +
            self._language_file.replace_comment_line('\n// --- /SIMPLIFY THIS ---', '//') +
            self._txt[self.cursor1:]
        )
        self.stop_sequences = self._language_file.replace_comment_line('\n// --- /SIMPLIFIED ---', '//')
        return prompt

    def _postprocess(self, completion: str) -> str:
        if completion.startswith('\n'):
            completion = completion[1:]

        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


# class ScratchpadMakeCodeShorterNCTX(ScratchpadCodex):
#     def __init__(self, **kwargs):
#         super().__init__(
#             **kwargs,
#             engine='code-davinci-002',
#             sp_function_name='make-code-shorter-nctx',
#         )
#
#     def _prompt(self) -> str:
#         prompt = self._pe_file.replace(
#             '{___place_code_here___}',
#             self._language_file.replace_comment_line(f'// language: {self._language_file.language}\n', '//') +
#             self._language_file.replace_comment_line('// --- SIMPLIFY THIS ---', '//') + '\n' +
#             self._selection +
#             self._language_file.replace_comment_line('\n// --- /SIMPLIFY THIS ---', '//')
#         )
#         self.stop_sequences = self._language_file.replace_comment_line('\n// --- /SIMPLIFIED ---', '//')
#         return prompt
#
#     def _postprocess(self, completion: str) -> str:
#         if completion.startswith('\n'):
#             completion = completion[1:]
#
#         return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


class ScratchpadPreciseNaming(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='precise-naming',
            stop_sequences=['\n"""', '\n\"\"\"']
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}', '//') + '\n' +
            self._selection
        )
        return prompt

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


class ScratchpadTimeComplexity(ScratchpadCodex):
    def __init__(self, **kwargs):
        super().__init__(
            **kwargs,
            engine='text-davinci-003',
            sp_function_name='time-complexity',
            stop_sequences=['\n"""', '\n\"\"\"']
        )

    def _prompt(self) -> str:
        prompt = self._pe_file.replace(
            '{___place_code_here___}',
            self._language_file.replace_comment_line(f'// language: {self._language_file.language}', '//') + '\n' +
            self._selection
        )
        return prompt

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor1] + completion + self._txt[self.cursor1:]
