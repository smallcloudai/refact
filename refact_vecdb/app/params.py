from collections import namedtuple
from typing import List

from pydantic import BaseModel
from typing import Tuple


class FindQuery(BaseModel):
    query: str
    top_k: int = 1


class FilesBulkUpload(BaseModel):
    files: List[Tuple[str, str]]
    step: int
    total: int


class VecDBUpdateProvider(BaseModel):
    provider: str
    batch_size: int


class DeleteFilesByNames(BaseModel):
    file_names: List[str]


FileUpload = namedtuple('FileUpload', ['name', 'text'])
