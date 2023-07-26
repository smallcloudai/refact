import os
from typing import Dict, Any, List

import requests
import ujson as json

import aiohttp


class SearchResult:
    def __init__(
            self,
            title: str,
            link: str,
            snippet: str,
            position: int
    ):
        self.title = title
        self.link = link
        self.snippet = snippet
        self.position = position

    def dict(self) -> Dict[str, Any]:
        return {
            'title': self.title,
            'link': self.link,
            'snippet': self.snippet,
            'position': self.position
        }


class WebSearch:
    def __init__(
            self,
            api_key: str = None,
            engine: str = 'google',
            top_n_results: int = 10,
    ):
        self._api_key = api_key or os.environ.get('SERP_API_KEY')
        assert self._api_key, f'api key for SerpSearch must be specified or provided as a SERP_API_KEY'
        self._engine = engine
        self._top_n_results = top_n_results
        self._search_url = 'https://google.serper.dev/search'
        self._timeout: float = 10

    def _headers(self) -> Dict[str, Any]:
        return {
            'Content-Type': 'application/json',
            'X-API-KEY': self._api_key
        }

    def _params(self, query: str) -> Dict[str, Any]:
        return {
            'q': query,
            'engine': self._engine
        }

    async def a_search(self, query: str) -> List[SearchResult]:
        async with aiohttp.ClientSession() as session:
            session.headers.update(self._headers())

            async with session.get(
                    self._search_url,
                    params=self._params(query),
                    timeout=self._timeout
            ) as resp:
                assert resp.status == 200, f'ERROR: {resp.text()}'
                data = json.loads(await resp.text())
                results = data['organic']
            return [SearchResult(
                title=r['title'],
                link=r['link'],
                snippet=r['snippet'],
                position=r['position']
            ) for r in results
            ][:self._top_n_results]

    def search(self, query) -> List[SearchResult]:
        response = requests.get(
            self._search_url,
            headers=self._headers(),
            params=self._params(query),
            timeout=self._timeout
        )
        response.raise_for_status()

        data = response.json()
        results = data['organic']

        return [SearchResult(
            title=r['title'],
            link=r['link'],
            snippet=r['snippet'],
            position=r['position']
        ) for r in results
        ][:self._top_n_results]


if __name__ == '__main__':
    search = WebSearch()
    print(json.dumps([s.dict() for s in search.search('hello world in python')], indent=4))
