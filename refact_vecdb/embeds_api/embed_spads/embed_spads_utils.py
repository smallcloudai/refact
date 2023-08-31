from typing import Iterable, List, Callable, Union


__all__ = ['ChunkifyFiles']


class ChunkifyFiles:
    def __init__(
            self,
            window_size: int,
            soft_limit: int,
            len_calc: Callable = lambda x: len(x)
    ):
        self._soft_window = window_size
        self._hard_window = window_size + soft_limit
        self._len_calc = len_calc

    def create(self, texts: Union[str, Iterable[str]]) -> Iterable[List[str]]:
        texts = texts if isinstance(texts, Iterable) else [texts]
        for text in texts:
            yield list(self._chunkify(text))

    def _chunkify(self, text: str) -> Iterable[str]:
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
                    batch_size += self._len_calc(stringify_batch(to_next_batch))
                    to_next_batch.clear()

                batch_size += self._len_calc(line)
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
