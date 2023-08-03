import os

from typing import Any, AsyncIterator, Dict, List, Iterator

import ujson as json

from refact_vecdb import VecDBAsyncAPI

__all__ = ['vecdb_call']


VECDB_URL = os.getenv('VECDB_URL', 'http://localhost:8008')


def vecdb_prompt(query: str, candidates: Any) -> str:
    candidates = json.dumps([{
        'short_name': c['file_name'],
        'snippet': c['text']
    } for c in candidates])

    return f"""
Here is an example of using {query}. I want you to understand how it is used:
{candidates}
Answer the two questions:
1. What is the purpose of {query}
2. Write a short example of usage {query} abstracting from the context
    """


def cut_candidates(
        enc: Any,
        candidates: List[Dict[str, Any]],
        max_tokens: int
) -> Iterator[
    Dict[str, Any]
]:
    tokens_left = max_tokens
    for c in candidates:
        tokens_left -= len(enc.encode(c['text']))
        yield c
        if tokens_left <= 0:
            break


async def vecdb_call(
        enc: Any,
        query: str,
        max_tokens: int = 1024
) -> AsyncIterator[
    Dict[str, Any]
]:
    vecdb = VecDBAsyncAPI(url=VECDB_URL)

    yield {
        "role": "assistant",
        "content": f"Querying vecdb for {query}",
        "gui_role": "tool_use",
    }

    candidates = list(cut_candidates(
        enc=enc,
        candidates=await vecdb.find(query, 10),
        max_tokens=max_tokens
    ))

    search_result = json.dumps([
        {
            'short_name': c['file_name'],
            'full_name': c['file_path'],
            'snippet': c['text']
        }
        for c in candidates
    ])

    yield {
        "role": "user",
        "content": vecdb_prompt(query, candidates),
        "gui_role": "documents",
        "gui_content": search_result,
        "gui_function": "vecdb",
    }

