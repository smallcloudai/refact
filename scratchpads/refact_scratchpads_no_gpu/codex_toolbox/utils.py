import json

from pathlib import Path
from typing import Optional, Tuple, List, Dict


class LanguageFile:
    def __init__(
            self,
            file_name: str
    ):
        self.file_name = file_name
        self.__get_language_called: bool = False
        self.__language: Optional[str] = None
        self.__lang2ext = None
        self.__ext2lang = None
        self.__lang2comment = None
        self.__comment_style_sing: Optional[str] = None
        self.__comment_style_mult: Optional[Tuple[str, str]] = None

        self._cursor_file_comment_style()

    @property
    def _lang2ext(self) -> Dict[str, List[str]]:
        if not self.__lang2ext:
            self.__lang2ext = json.loads(Path(__file__).parent.joinpath('misc', 'langdb-lang2ext.json').read_text())
        return self.__lang2ext

    @property
    def _ext2lang(self) -> Dict[str, str]:
        if not self.__ext2lang:
            self.__ext2lang = {ext: lang for lang, exts in self._lang2ext.items() for ext in exts}
        return self.__ext2lang

    @property
    def _lang2comment(self) -> Dict[str, List]:
        if not self.__lang2comment:
            self.__lang2comment = json.loads(
                Path(__file__).parent.joinpath('misc', 'langdb-lang2comment.json').read_text())
        return self.__lang2comment

    @property
    def language(self) -> Optional[str]:
        if not self.__get_language_called:
            if '.' not in self.file_name:
                language = self._ext2lang.get(self.file_name)
            else:
                language = self._ext2lang.get("." + self.file_name.split('.')[-1])

            if not language and '.' in self.file_name:
                language = self._ext2lang.get(self.file_name)

            if not language:
                return

            assert isinstance(language, str), f"language is not a string: {language}; type: {type(language)}"
            self.__get_language_called = True
            self.__language = language

        return self.__language

    def _cursor_file_comment_style(self):
        language = self.language

        if not (comment_style := self._lang2comment.get(language)):
            self.__comment_style_sing = '//'
            return

        one_line_comment, multi_line_comment = comment_style

        if one_line_comment:
            self.__comment_style_sing = one_line_comment[0]

        if multi_line_comment:
            self.__comment_style_mult = tuple(multi_line_comment[:2])

    @property
    def comment_style_sing(self) -> Optional[str]:
        return self.__comment_style_sing

    @property
    def comment_style_mult(self) -> Optional[Tuple[str, str]]:
        return self.__comment_style_mult

    def replace_comment_line(self, line: str, old_comment: str) -> str:
        if old_comment not in line:
            return line

        if self.comment_style_sing:
            line = line.replace(old_comment, self.comment_style_sing)

        elif self.comment_style_mult:
            line = line.replace(old_comment, self.comment_style_mult[0]) + " " + self.comment_style_mult[1]

        return line

    def replace_comment_text(self, text: str, old_comment: str) -> str:
        return '\n'.join([self.replace_comment_line(line, old_comment) for line in text.splitlines()])
