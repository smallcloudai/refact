from typing import Dict, Any, List, Union, Iterable, Optional
from queue import Empty
from multiprocessing import Event, Process

from more_itertools import chunked

from refact_vecdb.embeds_api.context import CONTEXT as C
from refact_vecdb.embeds_api.encoder import VecDBEncoder


__all__ = ['VDBTextEncoderProcess']


def worker(
        q_in,
        q_out,
        event: Event,
        enc_params: Dict[str, Any]
) -> None:
    enc = VecDBEncoder(**enc_params)
    while True:
        event.wait()
        try:
            files = q_in.get(timeout=1)
            batch_size = 5
            q_out.put([
                {
                    'name': text_name,
                    'chunk': chunk,
                    'chunk_idx': chunks_batch_idx * batch_size + chunk_idx,
                    'embedding': embed
                }
                for text_chunks, text_name in zip(enc.chunkify(f['text'] for f in files), (f['name'] for f in files))
                for chunks_batch_idx, text_chunks_batch in enumerate(chunked(text_chunks, batch_size))
                for chunk_idx, (chunk, embed) in enumerate(zip(text_chunks_batch, enc.encode(text_chunks_batch)))
            ])
        except Empty:
            pass
        finally:
            event.clear()


class VDBTextEncoderProcess:
    def __init__(
            self,
            enc_params: Dict[str, Any]
    ):
        self._q_in = C.q_manager.Queue()
        self._q_out = C.q_manager.Queue()
        self._q_event = Event()
        self._process: Process = self._get_process(enc_params)

    def _get_process(self, enc_params: Dict[str, Any]) -> Process:
        process = Process(
            target=worker,
            args=(self._q_in, self._q_out, self._q_event, enc_params)
        )
        process.daemon = True
        process.start()
        return process

    def throw_task(self, files: Union[Dict, Iterable[Dict]]) -> None:
        files = [files] if isinstance(files, Dict) else list(files)
        self._q_in.put(files)
        self._q_event.set()

    def result(self) -> Optional[List[Dict[str, Any]]]:
        self._q_event.clear()
        result = self._q_out.get()
        return result
