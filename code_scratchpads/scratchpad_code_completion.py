from code_scratchpads import scratchpad_abstract

from typing import Dict, List, Any


class ScratchpadCodeCompletion(scratchpad_abstract.Scratchpad):
    def __init__(
        self,
        *,
        request_created_ts: float,
        tokenizer: Any,
        sources: Dict[str, List[str]],
        cursor_file: str,
        cursor_line: int,
        cursor_character: int,
        max_new_tokens: int,
        multiline: bool,
        supports_stop: bool,
    ):
        super().__init__(request_created_ts=request_created_ts, tokenizer=tokenizer)
        self.sources = sources
        self.cursor_file = cursor_file
        self.cursor_line = cursor_line
        self.cursor_character = cursor_character
        self.max_new_tokens = max_new_tokens
        self.multiline = multiline
        self.supports_stop = supports_stop
