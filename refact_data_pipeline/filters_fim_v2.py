import random
import re
from typing import Dict, Union, List, Optional
from typing import Tuple

import numpy as np
import termcolor
from refact_data_pipeline import DatasetOpts


def _line_sep_by_line(line: str):
    if line.endswith('\r\n'):
        return '\r\n'
    elif line.endswith('\n'):
        return '\n'
    elif line.endswith('\r'):
        return '\r'
    else:
        return ''


def _line_sep_size_by_line(line: str):
    line_sep = _line_sep_by_line(line)
    return len(line_sep)


def _softmax(x):
    return np.exp(x) / sum(np.exp(x))


def _random_trim_context(
        text: str,
        random: np.random.RandomState,
        min_rows: int = 6
) -> str:
    lines = text.splitlines(keepends=True)
    if len(lines) < min_rows:
        return text

    p1 = _softmax([np.log(len(lines) - i) ** 2 for i in range(len(lines))])
    p2 = list(reversed(p1))
    lines_indexes = list(range(len(lines)))
    cursor_1 = random.choice(lines_indexes, p=p1)
    cursor_2 = random.choice(lines_indexes, p=p2)
    cursor_1, cursor_2 = sorted((cursor_1, cursor_2))
    lines = lines[cursor_1:cursor_2]
    if len(lines) < min_rows:
        return text
    return "".join(lines)


class InsideSingleRow:
    def __init__(
            self,
            random: np.random.RandomState,
            retries: int = 3
    ):
        self._random = random
        self._retries = retries

    def __call__(
            self,
            lines: List[str],
            is_cut_file: bool
    ) -> Tuple[str, str, str]:
        retries = self._retries
        while retries:
            p = _softmax([np.log(len(row)) for row in lines])
            row_index = self._random.choice(list(range(len(lines))), p=p)
            selected_row = lines[row_index]
            if len(selected_row) < 3:
                retries -= 1
                continue
            low, high = np.sort(
                self._random.randint(0, len(selected_row) - _line_sep_size_by_line(selected_row), 2)
            )
            prefix = "".join(lines[:row_index]) + selected_row[:low]
            middle = selected_row[low:high]
            suffix = selected_row[high:] + "".join(lines[row_index + 1:])
            return prefix, middle, suffix
        else:
            raise RuntimeError("Could not found a split")


class MiddleToEndSingleRow:
    def __init__(
            self,
            random: np.random.RandomState,
            retries: int = 3
    ):
        self._random = random
        self._retries = retries

    def __call__(
            self,
            lines: List[str],
            is_cut_file: bool,
    ) -> Tuple[str, str, str]:
        retries = self._retries
        while retries:
            p = _softmax([np.log(len(row)) for row in lines])
            row_index = self._random.choice(list(range(len(lines))), p=p)
            selected_row = lines[row_index]
            if len(selected_row) < 1:
                retries -= 1
                continue
            low = self._random.randint(0, len(selected_row))
            high = len(selected_row) - _line_sep_size_by_line(selected_row)
            prefix = "".join(lines[:row_index]) + selected_row[:low]
            middle = selected_row[low:high]
            suffix = selected_row[high:] + "".join(lines[row_index + 1:])
            return prefix, middle, suffix
        else:
            raise RuntimeError("Could not found a split")


class MiddleToEndMultipleRows:
    def __init__(
            self,
            random: np.random.RandomState,
            max_rows_to_block_end: int = 32,
            retries: int = 3
    ):
        self._random = random
        self._max_rows_to_block_end = max_rows_to_block_end
        self._retries = retries
        self._is_empty_fn = re.compile(r'^[\s\r\n]*$')

    def __call__(
            self,
            lines: List[str],
            is_cut_file: bool,
    ) -> Tuple[str, str, str]:
        def _get_n_extra_rows_by_block(selected_row_idx) -> Optional[int]:
            low, high = selected_row_idx + 1, -1 if is_cut_file else len(lines)
            dist = next((idx for idx, line in enumerate(lines[low:high]) if self._is_empty_fn.match(line)), None)
            if dist is None or dist > self._max_rows_to_block_end:
                return None
            return dist + 1

        def _get_n_extra_rows(selected_row_idx) -> Optional[int]:
            low, high = 1, len(lines) - selected_row_idx - (1 if is_cut_file else 0)
            if high <= low:
                return None
            n_extra_lines = min(self._max_rows_to_block_end, self._random.randint(low, high))
            if n_extra_lines == 0:
                return 0
            else:
                return n_extra_lines

        retries = self._retries
        while retries:
            p = _softmax([np.log(len(row)) for row in lines])
            row_index = self._random.choice(list(range(len(lines))), p=p)
            selected_row = lines[row_index]
            if len(selected_row) < 1:
                continue

            n_extra_rows = _get_n_extra_rows_by_block(row_index)
            if n_extra_rows is None:
                n_extra_rows = _get_n_extra_rows(row_index)
            if n_extra_rows is None:
                retries -= 1
                continue

            low = self._random.randint(0, len(selected_row))
            last_line = lines[row_index + n_extra_rows]
            high = len(last_line) - _line_sep_size_by_line(selected_row)
            prefix = "".join(lines[:row_index]) + selected_row[:low]
            middle = selected_row[low:] + "".join(lines[row_index + 1:row_index + n_extra_rows]) + last_line[:high]
            suffix = last_line[high:] + "".join(lines[row_index + n_extra_rows + 1:])
            return prefix, middle, suffix
        else:
            raise RuntimeError("Could not found a split")


class EmptyMiddle:
    def __init__(
            self,
            random: np.random.RandomState,
    ):
        self._random = random
        self._extra_symbols = [' ', '\n']

    def __call__(
            self,
            lines: List[str],
            is_cut_file: bool,
    ) -> Tuple[str, str, str]:
        p = _softmax([np.log(len(row)) for row in lines])
        row_index = self._random.choice(list(range(len(lines))), p=p)
        line_sep = _line_sep_by_line(lines[row_index])
        selected_row = lines[row_index][:-len(line_sep)]
        row_symbol_index = self._random.randint(0, len(selected_row))
        extra_symbol = self._random.choice(self._extra_symbols)
        if extra_symbol == '\n':
            extra_symbol = line_sep
        prefix_selected_row = f'{selected_row[:row_symbol_index]}{extra_symbol}'
        suffix_selected_row = f'{selected_row[row_symbol_index:]}{line_sep}'
        prefix = "".join(lines[:row_index]) + prefix_selected_row
        middle = ""
        suffix = suffix_selected_row + "".join(lines[row_index + 1:])
        return prefix, middle, suffix


class FIMv2:
    def __init__(
            self,
            inner_filter,
            dataopts: DatasetOpts,
    ):
        self.inner_filter = inner_filter
        self.n_ctx = dataopts.get("n_ctx", 2048)
        self.fim_probability = dataopts.get("fim_probability", 0.5)
        self.tkr_stochastic_tokens = dataopts.get("tkr_stochastic_tokens", 3)
        self.fim_drop_residuals = bool(dataopts.get("fim_drop_residuals", 0))
        self.random_trim_context_prob = bool(dataopts.get("random_trim_context_prob", 0.0))
        self.debug = bool(dataopts.get("debug", 0))
        self.enc = dataopts.encoding
        self.enc.set_random_seed(dataopts.get("seed", 42))
        self.special_tokens = [
            self.enc.PREFIX,
            self.enc.SUFFIX,
            self.enc.INFILL,
            self.enc.EOT,
        ]
        assert len(set(self.special_tokens)) == len(self.special_tokens)
        self.random = np.random.RandomState(dataopts.get("seed", 42))
        self.splitters_probs = [
            (InsideSingleRow(random=self.random), 0.2),
            (MiddleToEndSingleRow(random=self.random), 0.399),
            (MiddleToEndMultipleRows(random=self.random), 0.4),
            (EmptyMiddle(random=self.random), 0.001)
        ]
        self.extra_payload_size = int(self.n_ctx * 0.03)

    def __iter__(self):
        stats: Dict[str, Union[int, float]] = {
            "fim_unicode_split": 0,
            "fim_unable_to_split": 0,
            "fim_out": 0,
            "fim_lowlines_skip": 0
        }
        for sample in self.inner_filter:
            text = sample["text"]
            if self.random.random() < self.random_trim_context_prob:
                text = _random_trim_context(text, self.random)
            tokens, _ = self.enc.encode_stochastic(text, [], 0.01 * self.tkr_stochastic_tokens)
            cursor = 0
            while cursor < len(tokens):
                if self.random.random() > self.fim_probability:
                    output_data, cursor = self._generate_plain_text(tokens, cursor, sample, stats)
                else:
                    output_data, cursor = self._generate_fim(tokens, cursor, sample, stats)
                if output_data is not None:
                    yield output_data

                if self.fim_drop_residuals:
                    break

    def _generate_plain_text(self, tokens, cursor, sample, stats) \
            -> Tuple[Optional[Dict[str, Union[str, List[str]]]], int]:
        plain = tokens[cursor: cursor + self.n_ctx]
        cursor += len(plain)
        is_cut_file = len(tokens[cursor:]) > 0
        mask = [1] * len(plain)
        plain.append(self.enc.EOT)
        # If last_chunk then the EOT is real, the model should predict it. If not, it just
        # acts as a separator, the model should not predict it.
        # And it's not visible anyway if len(plain) > n_ctx
        if is_cut_file:
            mask.append(0)
        else:
            mask.append(1)
        return {
            "tokens": plain,
            "mask": mask,
            "first": [1] + [0] * (len(plain) - 1),
            "stats": {**sample["stats"], **stats},
        }, cursor

    def _generate_fim(self, tokens, cursor, sample, stats) \
            -> Tuple[Optional[Dict[str, Union[str, List[str]]]], int]:
        payload_size = 0 if len(tokens) < (
                self.n_ctx - self.extra_payload_size) else self.extra_payload_size
        pre_fim_toks = tokens[cursor: cursor + self.n_ctx - payload_size]
        cursor += len(pre_fim_toks)
        is_cut_file = len(tokens[cursor:]) > 0
        try:
            text = self.enc.decode_utf8(pre_fim_toks)
        except:
            stats["fim_unicode_split"] += 1
            return None, cursor

        lines = text.splitlines(keepends=True)
        if len(lines) < 2:
            stats["fim_lowlines_skip"] += 1
            return None, cursor

        splitter_idx = self.random.choice(list(range(len(self.splitters_probs))),
                                          p=[p for _, p in self.splitters_probs])
        splitter = self.splitters_probs[splitter_idx][0]
        try:
            prefix, middle, suffix = splitter(lines=lines, is_cut_file=is_cut_file)
        except (RuntimeError, ValueError) as e:
            stats["fim_unable_to_split"] += 1
            return None, cursor

        prefix_toks, _ = self.enc.encode_stochastic(prefix, [], 0.01 * self.tkr_stochastic_tokens)
        suffix_toks, _ = self.enc.encode_stochastic(suffix, [], 0.01 * self.tkr_stochastic_tokens)
        if self.random.random() < 0.5:
            tokens_context = [self.enc.PREFIX] + prefix_toks + [self.enc.SUFFIX] + suffix_toks
            mask_context = [0] + [1] * len(prefix_toks) + [0] + [1] * len(suffix_toks)
        else:
            tokens_context = [self.enc.SUFFIX] + suffix_toks + [self.enc.PREFIX] + prefix_toks
            mask_context = [0] + [1] * len(suffix_toks) + [0] + [1] * len(prefix_toks)
        middle_toks, _ = self.enc.encode_stochastic(middle, [], 0.01 * self.tkr_stochastic_tokens)
        middle_mask = [1] * len(middle_toks)
        stats["fim_out"] += 1
        if self.debug:
            print(f'splitter: {splitter}, middle_size: {len(middle)}, middle: {middle}')
            print(termcolor.colored(self.enc.decode(prefix_toks), "red"), end='')
            print(termcolor.colored(self.enc.decode(middle_toks), "green"), end='')
            print(termcolor.colored(self.enc.decode(suffix_toks), "red"))
        return {
            "tokens": tokens_context + [self.enc.INFILL] + middle_toks + [self.enc.EOT],
            "mask": mask_context + [0] + middle_mask + [1],
            "first": [1] + [0] * (-1 + len(tokens_context) + 1 + len(middle_toks) + 1),
            "stats": {**sample["stats"], **stats},
        }, cursor
