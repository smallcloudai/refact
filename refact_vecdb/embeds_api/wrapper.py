
from typing import List, Dict, Union, Iterable, AsyncIterator, Iterator

import requests
import aiohttp
import ujson as json


__all__ = ['VDBEmbeddingsAPI']


class VDBEmbeddingsAPI:
    def __init__(
            self,
            url: str = 'http://localhost:8882'
    ):
        self._url = url

    def _headers(self) -> Dict:
        return {
            'Content-Type': 'application/json',
        }

    async def providers(self) -> List[str]:
        async with aiohttp.ClientSession(headers=self._headers()) as session:
            async with session.get(f'{self._url}/v1/providers') as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                body = await resp.json()
                return body['providers']

    async def a_create(
            self,
            texts: Union[Dict, Iterable[Dict]],
            provider: str,
            is_index: str = 'False'
    ) -> AsyncIterator[Dict[str, str]]:
        texts = [texts] if isinstance(texts, Dict) else list(texts)
        async with aiohttp.ClientSession(headers=self._headers()) as session:
            async with session.post(
                f'{self._url}/v1/embed',
                params={'provider': provider, 'is_index': str(is_index)},
                json=texts
            ) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                async for chunk in resp.content.iter_any():
                    if chunk:
                        yield json.loads(chunk)

    def create(
            self,
            texts: Union[Dict, Iterable[Dict]],
            provider: str,
            is_index: str = 'False'
    ) -> Iterator[Dict[str, str]]:
        texts = [texts] if isinstance(texts, Dict) else list(texts)

        response = requests.post(
            f'{self._url}/v1/embed',
            headers=self._headers(),
            params={'provider': provider, 'is_index': str(is_index)},
            json=texts,
            stream=True
        )
        assert response.status_code == 200, f'Error: {response.text}'
        for chunk in response.iter_content(chunk_size=None):
            if chunk:
                yield json.loads(chunk)


if __name__ == '__main__':
    api = VDBEmbeddingsAPI()
    # import IPython; IPython.embed(); quit()
    print(list(api.create({'name': 'example', 'text': 'hello world'}, 'gte', False)))

