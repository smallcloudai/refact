import json
import time
import traceback

from typing import List, Iterator, Iterable, Union, Dict, Any

from more_itertools import chunked

from refact_vecdb.embeds_api.embed_spads import embed_providers, ChunkifyFiles

from refact_scratchpads_no_gpu.stream_results import UploadProxy


__all__ = ['VecDBEncoder']


class UnknownProviderException(Exception):
    def __init__(self, provider):
        self.provider = provider
        super().__init__(f"Unknown provider: {provider}; must be one of {embed_providers.keys()}")


class VecDBEncoder:
    def __init__(
            self,
            provider: str,
            batch_size: int = 5,
            **kwargs # noqa
    ):
        self._provider = provider
        self._batch_size = batch_size
        self._setup_encoder()
        self._setup_ch_files()

    def _setup_encoder(self) -> None:
        if self._provider not in embed_providers.keys():
            raise UnknownProviderException(self._provider)

        self._encoder = embed_providers[self._provider]()

    def _setup_ch_files(self):
        if self._provider == 'ada':
            self._ch_files = ChunkifyFiles(512, 512)
        elif self._provider == 'gte':
            self._ch_files = ChunkifyFiles(512, 512)

    def _encode(self, texts: Union[str, Iterable[str]]) -> Iterator[List[float]]:
        texts = [texts] if isinstance(texts, str) else texts
        yield from self._encoder.create(texts)

    def _chunkify(self, texts: Union[str, Iterable[str]]) -> Iterator[List[str]]:
        texts = [texts] if isinstance(texts, str) else texts
        yield from self._ch_files.create(texts)

    def _process_files_batch(
            self,
            files_batch: List[Dict],
    ) -> Dict:
        res = [
            {
                'name': text_name,
                'chunk': chunk,
                'chunk_idx': chunks_batch_idx * self._batch_size + chunk_idx,
                'embedding': embed
            }
            for text_chunks, text_name in
            zip(self._chunkify(f['text'] for f in files_batch), (f['name'] for f in files_batch))
            for chunks_batch_idx, text_chunks_batch in enumerate(chunked(text_chunks, self._batch_size))
            for chunk_idx, (chunk, embed) in enumerate(zip(text_chunks_batch, self._encode(text_chunks_batch)))
        ]
        return {idx: json.dumps(r) for idx, r in enumerate(res)}

    def infer(self, request: Dict[str, Any], upload_proxy: Union[UploadProxy, Any], upload_proxy_args: Dict, log=print):
        request_id = request["id"]
        try:
            upload_proxy_args["ts_prompt"] = time.time()
            if request_id in upload_proxy.check_cancelled():
                return

            files = self._process_files_batch(request["files"])

            upload_proxy_args["ts_batch_finished"] = time.time()
            finish_reason = 'DONE'
            upload_proxy.upload_result(
                **upload_proxy_args,
                files=[files],
                finish_reason=[finish_reason],
                generated_tokens_n=[0],
                more_toplevel_fields=[{}],
                status="completed",
            )

        except Exception: # noqa
            log(f"Error while processing request {request_id}")
            log(traceback.format_exc())
