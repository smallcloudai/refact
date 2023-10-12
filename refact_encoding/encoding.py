import random
import numpy as np

from copy import copy
from pathlib import Path
from typing import List, Tuple

import tiktoken
from tiktoken.load import load_tiktoken_bpe


__all__ = ["RefactEncoding"]


class RefactEncoding:
    def __init__(self, name: str, random_seed: int = 42):
        self.DIAMOND = 0
        self.INFILL = 0
        self.ESCAPE = 0
        self.MSG = 0
        self.FILE = 0
        self.CHUNK = 0
        self.LF = 0
        self.LFLF = 0
        self.EOT = 0
        self.DUMMY = 0
        self.PREFIX = 0
        self.SUFFIX = 0
        self._pos_tokens = []
        self._tokenizer = None
        self._sentencepiece_tokenizer = None
        self._allowed_special = set()
        self._slash_n_banlist = set()
        self._random = random.Random(random_seed)

        if name in ["openai_reversible50000"]:
            self.EOT = 50256
            pat_str = r"""'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+"""
            special_tokens = {
                "<|endoftext|>": 50256,
            }
            mergeable_ranks = load_tiktoken_bpe("az://openaipublic/encodings/r50k_base.tiktoken")
            self._tik = tiktoken.Encoding(
                name,
                pat_str=pat_str,
                mergeable_ranks=mergeable_ranks,
                special_tokens=special_tokens,
            )
            self.n_vocab = self._tik.n_vocab
            assert self.n_vocab == 50257
            self.LF = self._encode_token("\n")
            assert self.LF == 198
            self.LFLF = self._encode_token("\n\n")

        elif name in ["openai_cl100k", "openai_programming_v2"]:
            if name == "openai_cl100k":
                tt_name = "az://openaipublic/encodings/cl100k_base.tiktoken"
                pat_str = r"""(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}{1,3}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+"""
                special_tokens = {
                    "<|unknown0|>": 100256,
                    "<|endoftext|>": 100257,
                    "<|fim_prefix|>": 100258,
                    "<|fim_middle|>": 100259,
                    "<|fim_suffix|>": 100260,
                    # "<|endofprompt|>": 100276,
                    "►": 100261,  #100277,
                    "●": 100262,  #100278,
                    "§": 100263,  #100279,
                }
                self.EOT = 100257
                self.INFILL = 100259
                self.DUMMY = 100260
                self.CHUNK = 100261
                self.DIAMOND = 100262
                self.ESCAPE = 100263
                self._allowed_special = set(["<|fim_middle|>", "<|fim_suffix|>"])
                last_special_plus_one = 100264
            elif name == "openai_programming_v2":
                pat_str = r"""'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+"""
                tt_name = "az://openaipublic/encodings/p50k_base.tiktoken"
                special_tokens = {
                    "<|endoftext|>": 50256,
                    # "  ": 50257
                    # ...space tokens...
                    # "                         ": 50280,
                }
                self.EOT = 50256
                self.ESCAPE = 47171   # " §§"
                self.INFILL = 25992   # " 裏覚醒"
                self.DIAMOND = 48049  # " ●"
                self.CHUNK = 34933    # " ►"
                self.DUMMY = self.DIAMOND    # something that can be converted to text and back to token
                last_special_plus_one = 50281
            else:
                assert 0
            chars = "ABCD"
            position_tokens = ["⪦" +
                    chars[i//4//4//4//4 % 4] +
                    chars[i//4//4//4 % 4] +
                    chars[i//4//4 % 4] +
                    chars[i//4 % 4] +
                    chars[i % 4] + "⪧"
                    for i in range(1024)]
            for i, postok in enumerate(position_tokens):
                special_tokens[postok] = last_special_plus_one + i
            self._pos_tokens = list(range(last_special_plus_one, last_special_plus_one + 1024))

            mergeable_ranks = load_tiktoken_bpe(tt_name)
            self._tik = tiktoken.Encoding(
                name,
                pat_str=pat_str,
                mergeable_ranks=mergeable_ranks,
                special_tokens=special_tokens,
            )
            self.n_vocab = self._tik.n_vocab
            if name == "openai_cl100k":
                assert self.n_vocab == 101288
            elif name == "openai_programming_v2":
                assert self.n_vocab == 51305
            else:
                assert 0
            self.MSG = self._encode_token(" MSG")
            self.FILE = self._encode_token(" FILE")
            self.LF = self._encode_token("\n")
            assert self.LF == 198
            self.LFLF = self._encode_token("\n\n")
            LEAVE_LESS_TPOS = 256
            self._pos_tokens = self._pos_tokens[:LEAVE_LESS_TPOS]
            for i in range(self._tik.n_vocab):
                if i==198:  # Only allow one token with \n
                    continue
                if "\n" in self._tik.decode([i]):
                    self._slash_n_banlist.add(i)
                    # print("%05i \"%s\"" % (i, self._tik.decode([i]).replace("\n", "\\n").replace("\r", "\\r")))

        elif name in ['llama']:
            from sentencepiece import SentencePieceProcessor
            filename = Path(__file__).resolve().parent / f"{name}.tokenizer.model"
            self._sentencepiece_tokenizer = SentencePieceProcessor(str(filename))
            self.n_vocab = self._sentencepiece_tokenizer.vocab_size()
            self.bos_id: int = self._sentencepiece_tokenizer.bos_id()
            self.DIAMOND = self._sentencepiece_tokenizer.unk_id()
            self.EOT = self._sentencepiece_tokenizer.eos_id()
            self.LF = 13

        elif name in ['bigcode_largemodel']:
            import tokenizers
            filename = Path(__file__).resolve().parent / f"{name}.json"
            self._tokenizer = tokenizers.Tokenizer.from_file(str(filename))
            self.DIAMOND = self.DUMMY = 4  # <fim-pad>
            self.FILE = 5  # <filename>
            self.EOT = 0  # <|endoftext|>
            self.INFILL = 2  # <fim-middle>
            self.PREFIX = 1  # <fim-prefix>
            self.SUFFIX = 3  # <fim-suffix>
            self.ESCAPE = 14  # originally was <empty_output>
            self.MSG = 16  # "<commit_msg>"
            # unique tokens
            self.GH_STARS = 6  # "<gh_stars>"
            self.ISSUE_START = 7  # <issue_start>
            self.ISSUE_COMMENT = 8  # "<issue_comment>"
            self.ISSUE_CLOSED = 9  # "<issue_closed>"
            self.JUPYTER_START = 10  # "<jupyter_start>"
            self.JUPYTER_TEXT = 11  # "<jupyter_text>"
            self.JUPYTER_CODE = 12  # "<jupyter_code>"
            self.JUPYTER_OUTPUT = 13  # "<jupyter_output>"
            self.EMPTY_OUTPUT = 14  # "<empty_output>"
            self.COMMIT_BEFORE = 15  # "<commit_before>"
            self.COMMIT_AFTER = 17  # "<commit_after>"
            self.REPONAME = 18  # "<reponame>"
            self.LF = self._encode_token("\n")
            self.LFLF = self._encode_token("\n\n")
            self.CURSOR = self._encode_token("CURSOR")
            self.n_vocab = self._tokenizer.get_vocab_size()

        else:
            assert 0, "unknown encoding %s" % name

        if self._tokenizer is not None:
            # NOTE: this is workaround for huggingface tokenizer
            self._token2text = dict()
            for t in range(self.n_vocab):
                self._token2text[t] = self._tokenizer.decode([t])
            self._replacement_char = "�"
            self._replacement_char_token = self._encode_token(self._replacement_char)
        elif self._sentencepiece_tokenizer is not None:
            self._token2text = dict()
            for t in range(self.n_vocab):
                self._token2text[t] = self._sentencepiece_tokenizer.decode([t])
        else:
            self._token2bytes = dict()
            for t in range(self.n_vocab):
                self._token2bytes[t] = self._tik.decode_bytes([t])

    def set_random_seed(self, random_seed: int):
        self._random = random.Random(random_seed)

    def decode_utf8(self, tokens) -> str:
        if self._tokenizer:
            if len(tokens) == 1:
                if self._replacement_char in self._token2text[tokens[0]] and tokens[0] != self._replacement_char_token:
                    raise UnicodeDecodeError
                return self._token2text[tokens[0]]
            else:
                text = self.decode(tokens)
                if self._replacement_char in self._token2text[tokens[0]]:
                    raise UnicodeDecodeError
                return text
        else:
            text_bytes = b"".join([self._token2bytes[t] for t in tokens])
            return text_bytes.decode("utf8")

    def _encode_token(self, text: str) -> int:
        if self._tokenizer:
            tokens = self._tokenizer.encode(text).ids
        elif self._sentencepiece_tokenizer:
            tokens = self._sentencepiece_tokenizer.encode(text)
        else:
            tokens = self._tik.encode_ordinary(text)
        assert len(tokens) == 1, (text, tokens)
        return tokens[0]

    @property
    def tpos(self) -> List[int]:
        return copy(self._pos_tokens)

    def is_tpos(self, token: int) -> bool:
        if not self._pos_tokens:
            return False
        return self._pos_tokens[0] <= token <= self._pos_tokens[-1]

    def encode(self, txt: str) -> List[int]:
        if self._tokenizer:
            return self._tokenizer.encode(txt).ids  #, add_special_tokens=False)
        elif self._sentencepiece_tokenizer:
            return [self.bos_id] + self._sentencepiece_tokenizer.encode(txt)
        else:
            result = []
            cursor = 0
            while 1:
                slash_n = txt.find("\n", cursor)
                if slash_n == -1:
                    more = self._tik.encode(txt[cursor:], allowed_special=self._allowed_special, disallowed_special=())
                    result.extend(more)
                    break
                else:
                    more = self._tik.encode(txt[cursor:slash_n], allowed_special=self._allowed_special, disallowed_special=())
                    result.extend(more)
                    result.append(self.LF)
                cursor = slash_n + 1
            return result

    def encode_stochastic(self, sequence, bounds_at: List[int], prob: float) -> Tuple[List[int], List[int]]:
        bounds_n = int(len(sequence) * prob)
        if len(bounds_at) > 0:
            assert bounds_at[0] == 0
            assert bounds_at[-1] == len(sequence)
            bounds_at = list(set(bounds_at))
            bounds_at.sort()
        else:
            bounds_set = set([self._random.randint(0, len(sequence) - 1)
                              for _ in range(bounds_n)])
            bounds_set.add(len(sequence))
            bounds_set.add(0)
            bounds_at = list(bounds_set)
            bounds_at.sort()
        if len(bounds_at) == 1:  # set() eats equal elements, bad for zero-length strings
            bounds_at = [0, len(sequence)]
        result = []
        for a, b in zip(bounds_at[:-1], bounds_at[1:]):
            result.extend(self.encode(sequence[a:b]))
        return result, bounds_at

    def decode(self, tokens, skip_zeros: bool = False, cut_at_eot: bool = False) -> str:
        if isinstance(tokens, np.ndarray):
            assert len(tokens.shape) == 1, tokens.shape
        else:
            tokens = np.array(tokens)
        if skip_zeros:
            i = np.argmax(tokens > 0)
            tokens = tokens[i:]
        if cut_at_eot:
            i = np.argmax(tokens == self.EOT)
            if i > 0:
                tokens = tokens[:i]
        if self._tokenizer:
            return self._tokenizer.decode(tokens, skip_special_tokens=False)
        elif self._sentencepiece_tokenizer:
            return self._sentencepiece_tokenizer.decode(tokens.tolist())
        else:
            return self._tik.decode(tokens)

