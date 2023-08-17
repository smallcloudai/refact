from typing import Any, List, Iterator, Iterable, Union

from refact_vecdb.app.context import CONTEXT as C
from refact_vecdb.app.embed_spads import embed_providers


class UnknownProviderException(Exception):
    def __init__(self, provider):
        self.provider = provider
        super().__init__(f"Unknown provider: {provider}; must be one of {embed_providers.keys()}")


class VecDBEncoder:
    def __init__(self):
        self._provider = C.provider
        self._checkup()
        self._setup_encoder()

    def encode(self, texts: Union[str, Iterable[str]]) -> Iterator[List[float]]:
        texts = [texts] if isinstance(texts, str) else texts
        if self._provider == 'ada':
            for text in texts:
                yield self._encoder.create(text)

        elif self._provider == 'gte':
            yield from self._encoder.create(texts)

        else:
            raise UnknownProviderException(self._provider)

    def _checkup(self) -> None:
        if self._provider not in embed_providers.keys():
            raise UnknownProviderException(self._provider)

    def _setup_encoder(self) -> None:
        self._encoder = embed_providers[self._provider]()


