import os
import sys
import termcolor
import functools

from pathlib import Path
from typing import Callable, Union, List, Dict, Iterator

import openai
import tiktoken

from more_itertools import chunked
from tiktoken import Encoding

from .utils import LanguageFile

openai.api_key = os.environ.get("OPENAI_API_KEY")
ACCUMULATE_N_STREAMING_CHUNKS = 5

DEBUG = int(os.environ.get("DEBUG", "0"))


@functools.lru_cache(maxsize=10)
def engine_to_encoding(engine: str) -> Encoding:
    enc = tiktoken.encoding_for_model(engine)
    return enc


engine_to_encoding("text-davinci-003")   # this immediately tests if tiktoken works or not


class ScratchpadCodex:
    def __init__(
            self,
            id: str,
            created: float,
            temperature: float,
            max_tokens: int,
            stop_tokens: Union[str, List[str]],
            function: str,
            intent: str,
            cursor_file: str,
            cursor0: int,
            cursor1: int,
            max_edits: int,
            sources: Dict[str, str],
            stream: bool,
            logger: Callable,
            engine: str,
            sp_function_name: str,
            stop_sequences: Union[str, List[str]] = None,
            **unused,
    ):
        self.id = id
        self.created = created
        self.finish_reason = ""
        self.temp = min(max(float(temperature), 0.0), 1.0)
        self.max_tokens = int(max_tokens)
        self.function = function
        self.intent = intent
        self.cursor_file = cursor_file
        self.cursor0 = cursor0
        self.cursor1 = cursor1
        self.max_edits = max_edits
        self.sources = sources
        self.stream = stream
        self._logger = logger
        tmp = stop_tokens
        if isinstance(tmp, str):
            stop_strings = [tmp]
        else:
            stop_strings = tmp
        for k, v in unused.items():
            self.debuglog("ScratchpadEcho: unused parameter '%s' = '%s'" % (k, v))
        self.metering_generated_tokens_n = 0
        self.metering_total_tokens_n = 0
        self.needs_upload = False

        self.function = sp_function_name

        # if sp_function_name != self.function:
        #     raise ValueError(f'function name {sp_function_name} does not match {self.function}')

        self._engine = engine
        self._stop_sequences = [stop_sequences] if isinstance(stop_sequences, str) else stop_sequences

        self._txt: str = self.sources[self.cursor_file]
        self._selection: str = self._txt[self.cursor0:self.cursor1]
        self._language_file = LanguageFile(self.cursor_file)

    @property
    def stop_sequences(self) -> List[str]:
        return self._stop_sequences

    @stop_sequences.setter
    def stop_sequences(self, stop_sequences: Union[str, List[str]]):
        self._stop_sequences = [stop_sequences] if isinstance(stop_sequences, str) else stop_sequences

    @property
    def _pe_file(self) -> str:
        prompt = self._language_file.replace_comment_text(
            Path(__file__).parent.joinpath(f'prompts/{self.function}').read_text(), "//"
        )
        return prompt

    def _openai_completion(
            self,
            engine: str,
            prompt: str,
            stream: bool,
    ) -> openai.Completion:
        return openai.Completion.create(
            engine=engine,
            prompt=prompt,
            temperature=self.temp,
            max_tokens=self.max_tokens,
            stream=stream,
            stop=self.stop_sequences,
            logprobs=0,  # "tokens": ["Say"," this"," is"," a"," test"], "token_logprobs":[null,-4.5408506,-2.4766643,-1.41573]
        )

    def completion(
            self,
            final: bool,
            accumulate_n_streaming_chunks: int = 0,
    ) -> Iterator[Dict[str, str]]:
        if final:
            accumulate_n_streaming_chunks = self.max_tokens
        elif not final and accumulate_n_streaming_chunks == 0:
            accumulate_n_streaming_chunks = ACCUMULATE_N_STREAMING_CHUNKS
        prompt = self._prompt()
        if DEBUG:
            sys.stdout.write(termcolor.colored(prompt, "white", "on_grey", ["dark"]))
            sys.stdout.flush()
        completion: openai.Completion = self._openai_completion(self._engine, prompt=prompt, stream=self.stream)
        completion_so_far: str = ""
        openai_counter_total_tokens = 0
        if not self.stream:
            usage = completion["usage"]
            # {
            # "completion_tokens": 29,
            # "prompt_tokens": 1169,
            # "total_tokens": 1198
            # }
            openai_counter_total_tokens = usage["total_tokens"]
            completion_so_far = completion["choices"][0]["text"]
            self.finish_reason = completion["choices"][0]["finish_reason"]
            if DEBUG:
                sys.stdout.write(termcolor.colored(completion_so_far, "green"))
                sys.stdout.flush()
            if self.finish_reason is None:
                self.finish_reason = "END"
        else:
            for resp in chunked(completion, accumulate_n_streaming_chunks):
                addition = "".join([r.choices[0].text for r in resp])
                if DEBUG:
                    sys.stdout.write(termcolor.colored(addition, "green"))
                    sys.stdout.flush()
                completion_so_far += addition
                self.finish_reason = resp[-1].choices[0].finish_reason
                if self.finish_reason:
                    break
                modified = self._postprocess(completion_so_far)
                yield {self.cursor_file: modified}
        if self.finish_reason is None:
            self.finish_reason = "END"
        enc = engine_to_encoding(self._engine)
        self.metering_generated_tokens_n = len(enc.encode(completion_so_far, disallowed_special=()))
        self.metering_total_tokens_n = len(enc.encode(prompt, disallowed_special=())) + self.metering_generated_tokens_n
        if DEBUG:
            sys.stdout.write("\n")
            msg = "OPENAI METERING %i TOTAL TOKENS\n" % openai_counter_total_tokens
            msg += "TIKTOKEN METERING %i TOTAL TOKENS" % self.metering_total_tokens_n
            sys.stdout.write(termcolor.colored(msg, "red") + "\n")
            sys.stdout.flush()
            msg = "REASON STOPPED: \"%s\"" % self.finish_reason
            sys.stdout.write(termcolor.colored(msg, "red") + "\n")
            sys.stdout.flush()
        modified = self._postprocess(completion_so_far)
        yield {self.cursor_file: modified}

    def _postprocess(self, text: str) -> str:
        raise NotImplementedError

    def _prompt(self) -> str:
        raise NotImplementedError

    def toplevel_fields(self):
        if not self.finish_reason:
            return {}
        else:
            return {
                "metering_total_tokens": self.metering_total_tokens_n,
                "metering_generated_tokens": self.metering_generated_tokens_n,
                "metering_pp1000t": 2*1000,   # davinci $0.02, the same as $0.02*100000
            }
            # 400000 points is $4*100000, current davinci price $4/0.02 = 200k tokens

    def debuglog(self, *args):
        if self._logger:
            self._logger(*args)
