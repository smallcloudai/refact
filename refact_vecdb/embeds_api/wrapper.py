
from typing import Dict, Union, Iterable, AsyncIterator, Iterator

import requests
import aiohttp
import ujson as json


__all__ = ['VDBEmbeddingsAPI']


class VDBEmbeddingsAPI:
    def __init__(
            self,
            url: str = 'http://localhost:8008'
    ):
        self._url = url

    def _headers(self) -> Dict:
        return {
            'Content-Type': 'application/json',

        }

    async def a_create(
            self,
            texts: Union[Dict, Iterable[Dict]],
            provider: str,
    ) -> Iterable[Dict[str, str]]:
        texts = [texts] if isinstance(texts, Dict) else list(texts)
        async with aiohttp.ClientSession(headers=self._headers()) as session:
            async with session.post(
                f'{self._url}/v1/embeddings',
                json={'model': provider, 'files': texts},
            ) as resp:
                assert resp.status == 200, f'Error: {await resp.text()}'
                j = await resp.json()
                return j['data']

    def create(
            self,
            texts: Union[Dict, Iterable[Dict]],
            provider: str,
    ) -> Iterable[Dict[str, str]]:
        texts = [texts] if isinstance(texts, Dict) else list(texts)

        response = requests.post(
            f'{self._url}/v1/embeddings',
            headers=self._headers(),
            json={'model': provider, 'files': texts},
            stream=True,
            timeout=60
        )
        assert response.status_code == 200, f'Error: {response.text}'
        data = response.json()['data']
        return data


if __name__ == '__main__':
    api = VDBEmbeddingsAPI()
    # import IPython; IPython.embed(); quit()
    print(list(api.create({'name': 'example', 'text': 'hello world'}, 'gte')))
