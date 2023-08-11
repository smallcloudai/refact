
from math import ceil
from pathlib import Path
from collections import namedtuple
from typing import Iterable, Tuple, Optional, Union, List, Dict

import requests
import aiohttp
import ujson as json

from tqdm import tqdm
from more_itertools import chunked


FileUpload = namedtuple('FileUpload', ['name', 'text'])


class VecDBAPI:
    def __init__(
            self,
            url: str = 'http://0.0.0.0:8009',
            api_key: str = ''
    ):
        self._base_url = url
        self._api_key = api_key

        self._r_session = requests.Session()
        self._r_session.headers.update(self._headers())

    def _headers(self) -> Dict:
        return {
            'Content-Type': 'application/json',
            'X-Auth-Token': self._api_key
        }

    def find(
            self,
            query: str,
            top_k: int = 1
    ) -> List[Dict]:
        if isinstance(query, str):
            file = FileUpload(text=query, name='')
        else:
            raise ValueError(f'file should be str; got: {query}; type: {type(query)}')

        resp = self._r_session.post(
            f'{self._base_url}/v1/find',
            json={
                'query': file.text,
                'top_k': top_k
            },
            timeout=60
        )
        assert resp.status_code == 200, f'Error: {resp.text}'
        return resp.json()['results']

    def upload_files(
            self,
            files: Union[
                Iterable[Tuple[str, str]],
                Iterable[Path],
                Path,
            ],
            batch_size: int = 10
    ):
        files = self._resolve_files(files)

        total = ceil(len(files) / batch_size)
        for idx, files_batch in enumerate(tqdm(
                chunked(files, batch_size),
                total=total,
                desc='[VECDB]: Uploading files',
        )):
            data = {
                'files': [(f.name, f.text) for f in files_batch]
            }
            if idx == total - 1:
                data['final'] = True
            self._r_session.post(
                f'{self._base_url}/v1/bulk_upload',
                json=data,
                timeout=6 * batch_size
            )

    def _resolve_files(
            self,
            files: Union[
                Iterable[Tuple[str, str]],
                Iterable[Path],
                Path,
            ],
    ) -> List[FileUpload]:
        files: List[Union[Tuple, Path]] = [files] if isinstance(files, Path) else list(files)
        if not files:
            raise ValueError('files should not be empty')

        if isinstance(files, List):
            if isinstance(files[0], Tuple):
                files: List[FileUpload] = [FileUpload(name=name, text=text) for name, text in files]

            elif isinstance(files[0], Path):
                files_dirs = [d for d in files if d.is_dir()]
                files_files = [f for f in files if f.is_file()]
                files: List[FileUpload] = [
                    FileUpload(name=str(file), text=self._read_file(file))
                    for file in [
                        *files_files,
                        *[file for file_dir in files_dirs for file in file_dir.rglob('*') if file.is_file()]
                    ]
                ]
                files = [file for file in files if file[1]]
            else:
                raise ValueError(f'files should be list of tuples or paths; got files[0] type: {type(files[0])}')

        return files

    @staticmethod
    def _read_file(file: Path) -> Optional[str]:
        try:
            return file.read_text()
        except UnicodeDecodeError:
            return


class VecDBAsyncAPI(VecDBAPI):
    def __init__(
            self,
            *args, **kwargs
    ):
        super().__init__(*args, **kwargs)

    async def health(self):
        async with aiohttp.ClientSession() as session:
            session.headers.update(self._headers())
            async with session.get(
                f'{self._base_url}/v1/health',
                timeout=3
            ) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                data = json.loads(await resp.text())
                return data

    async def files_stats(self):
        async with aiohttp.ClientSession() as session:
            session.headers.update(self._headers())
            async with session.get(
                f'{self._base_url}/v1/files-stats',
                timeout=3
            ) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                data = json.loads(await resp.text())
                return data

    async def delete_all_records(self):
        async with aiohttp.ClientSession() as session:
            session.headers.update(self._headers())
            async with session.get(f'{self._base_url}/v1/delete_all', timeout=10) as resp:
                assert resp.status == 200, f'Error: {resp.text}'

    async def find(
            self,
            query: str,
            top_k: int = 1
    ) -> List[Dict]:
        if isinstance(query, str):
            file = FileUpload(text=query, name='')
        else:
            raise ValueError(f'query should be str; got: {query}; type: {type(query)}')

        async with aiohttp.ClientSession() as session:
            session.headers.update(self._headers())

            async with session.post(
                f'{self._base_url}/v1/find',
                json={
                    'query': file.text,
                    'top_k': top_k
                },
                timeout=60
            ) as resp:
                assert resp.status == 200, f'Error: {resp.text}'
                data = json.loads(await resp.text())
                return data['results']  # file_path, file_name, text

    async def upload_files(
            self,
            files: Union[
                Iterable[Tuple[str, str]],
                Iterable[Path],
                Path,
            ],
            batch_size: int = 10
    ):
        files = self._resolve_files(files)
        total = ceil(len(files) / batch_size)

        async with aiohttp.ClientSession() as session:
            session.headers.update(self._headers())

            for idx, files_batch in enumerate(tqdm(
                    chunked(files, batch_size),
                    total=total,
                    desc='[VECDB]: Uploading files',
            )):
                data = {
                    'files': [(str(f.name), f.text) for f in files_batch]
                }
                if idx == total - 1:
                    data['final'] = True
                async with session.post(
                    f'{self._base_url}/v1/bulk_upload',
                    json=data,
                    timeout=6 * batch_size
                ) as resp:
                    pass


if __name__ == '__main__':
    vecdb = VecDBAPI(
        url='http://0.0.0.0:8009',
    )
    vecdb.upload_files(
        files=Path('/Users/valaises/PycharmProjects/data-collection/github/scripts'),
        batch_size=10
    )
    # res = vecdb.find(
    #     'ParallelTasks',
    #     top_k=1
    # )
    # print(res[0]['text'])
