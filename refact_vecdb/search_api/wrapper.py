
from typing import Dict, Union, Iterable, AsyncIterator, Iterator

import requests
import aiohttp
import ujson as json


__all__ = ['VDBSearchAPI']


class VDBSearchAPI:
    def __init__(
            self,
            url: str = 'http://localhost:8883'
    ):
        self._url = url

    def _headers(self) -> Dict:
        return {
            'Content-Type': 'application/json',
        }

    async def status(self, account: str) -> Dict:
        async with aiohttp.ClientSession(headers=self._headers()) as session:
            async with session.post(f'{self._url}/v1/status', json={'account': account}) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                return json.loads(await resp.text())

    async def files_stats(self, account: str) -> Dict:
        async with aiohttp.ClientSession(headers=self._headers()) as session:
            async with session.post(f'{self._url}/v1/files-stats', json={'account': account}) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                return json.loads(await resp.text())

    async def a_search(
            self,
            texts: Union[str, Iterable[str]],
            account: str,
            top_k: int = 3
    ) -> AsyncIterator[Dict[str, str]]:
        texts = [texts] if isinstance(texts, str) else list(texts)

        async with aiohttp.ClientSession(headers=self._headers()) as session:
            async with session.post(
                    f'{self._url}/v1/search',
                    json={'texts': texts, 'account': account, 'top_k': top_k}
            ) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                async for chunk in resp.content.iter_any():
                    if chunk:
                        yield json.loads(chunk)

    def search(
            self,
            texts: Union[str, Iterable[str]],
            account: str,
            top_k: int = 3
    ) -> Iterator[Dict[str, str]]:
        texts = [texts] if isinstance(texts, str) else list(texts)

        response = requests.post(
            f'{self._url}/v1/search',
            headers=self._headers(),
            json={'texts': texts, 'account': account, 'top_k': top_k}
        )
        assert response.status_code == 200, f'Error: {response.text}'
        for chunk in response.iter_content(chunk_size=None):
            if chunk:
                yield json.loads(chunk)


if __name__ == '__main__':
    api = VDBSearchAPI()
    print(list(api.search('hello world', 'default')))
