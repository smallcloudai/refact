from typing import List, Iterator, Iterable, Union

from refact_vecdb.embeds_api.embed_spads import embed_providers, ChunkifyFiles


__all__ = ['VecDBEncoder']


class UnknownProviderException(Exception):
    def __init__(self, provider):
        self.provider = provider
        super().__init__(f"Unknown provider: {provider}; must be one of {embed_providers.keys()}")


class VecDBEncoder:
    def __init__(
            self,
            provider: str,
            **kwargs
    ):
        self._provider = provider
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

    def encode(self, texts: Union[str, Iterable[str]]) -> Iterator[List[float]]:
        texts = [texts] if isinstance(texts, str) else texts
        yield from self._encoder.create(texts)

    def chunkify(self, texts: Union[str, Iterable[str]]) -> Iterator[List[str]]:
        texts = [texts] if isinstance(texts, str) else texts
        yield from self._ch_files.create(texts)
