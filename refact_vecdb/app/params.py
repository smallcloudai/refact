from typing import List

from pydantic import BaseModel
from typing import Tuple


class FindQuery(BaseModel):
    query: str
    top_k: int = 1


class FilesBulk(BaseModel):
    files: List[Tuple[str, str]]
    final: bool = False
