from typing import Any, List, Optional, Iterable


class UnknownProviderException(Exception):
    def __init__(self, provider, providers):
        # ada or instructor
        self.provider = provider
        self.providers = providers
        super().__init__(f"Unknown provider: {provider}; must be one of {providers}")


class ChunkifyFiles:
    def __init__(
            self,
            window_size: int = 512,
            soft_limit: int = 256,
    ):
        self._soft_window = window_size
        self._hard_window = window_size + soft_limit

    def chunkify(self, text: str) -> Iterable[str]:
        if not text:
            yield ''
        else:
            def stringify_batch(batch: List[str]) -> str:
                return '\n'.join(batch)

            def find_sequence_start(list1, list2):
                for i in range(len(list2) - len(list1) + 1):
                    if all(list1[j] == list2[i + j] for j in range(len(list1))):
                        return i
                return -1

            batch_size = 0
            batch, soft_batch, to_next_batch = [], [], []
            for line in text.splitlines():
                line = line.rstrip()

                if to_next_batch:
                    batch = [*to_next_batch, *batch]
                    batch_size += len(stringify_batch(to_next_batch))
                    to_next_batch.clear()

                batch_size += len(line)
                if batch_size >= self._soft_window:
                    soft_batch.append(line)
                else:
                    batch.append(line)

                if batch_size >= self._hard_window:
                    lines_striped = [l.strip() for l in soft_batch]

                    best_break_line_n = find_sequence_start(['', ''], lines_striped)
                    if best_break_line_n == -1:
                        best_break_line_n = find_sequence_start([''], lines_striped)
                    if best_break_line_n == -1:
                        best_break_line_n = len(soft_batch) - 1

                    to_next_batch = soft_batch[best_break_line_n:]
                    soft_batch = soft_batch[:best_break_line_n]
                    batch = [*batch, *soft_batch]

                    yield stringify_batch(batch)

                    batch.clear()
                    soft_batch.clear()
                    batch_size = 0

            if batch or soft_batch:
                yield stringify_batch([*batch, *soft_batch])


class Encoder:
    def __init__(
            self,
            provider: str,
            instruction: Optional[str] = None,
    ):
        self._available_providers: List[str] = ['ada', 'instructor']
        self._provider = provider
        self._instruction = instruction

        self._checkup()
        self._setup_provider()

    def encode(self, text: str) -> Any:
        if self._provider == 'instructor':
            embedding = self._encoder.encode([[self._instruction, text]])[0]
            return embedding
        elif self._provider == 'ada':
            embedding = self._encoder.create(text)
            return embedding
        else:
            raise UnknownProviderException(self._provider, self._available_providers)

    def _checkup(self) -> None:
        if self._provider not in self._available_providers:
            raise UnknownProviderException(self._provider, self._available_providers)
        if self._provider == 'instructor' and not self._instruction:
            raise ValueError(f'instruction must be provided when using provider: {self._provider}')

    def _setup_provider(self) -> None:
        self._encoder: Any = None
        if self._provider == 'instructor':
            from InstructorEmbedding import INSTRUCTOR

            self._encoder = INSTRUCTOR('hkunlp/instructor-xl')
            # self._encoder.max_seq_len = self._window_size
        elif self._provider == 'ada':
            from refact_vecdb.app.embed_openai import OpenAIEmbedding

            self._encoder = OpenAIEmbedding('text-embedding-ada-002')
        else:
            raise UnknownProviderException(self._provider, self._available_providers)


