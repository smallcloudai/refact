from typing import Any, AsyncIterator, Dict, List, Iterator, Callable

import ujson as json

from refact_vecdb import VDBSearchAPI

__all__ = ['vecdb_call']


def vecdb_prompt(candidates: Any) -> str:
    candidates = json.dumps([{
        'short_name': c['file_name'],
        'snippet': c['text']
    } for c in candidates])

    return f"""
Here are examples that you will use to answer the questions above. 
{candidates}
Having these examples in mind, answer the question I asked you before.
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
        max_tokens: int = 1024,
        prompt: Callable = vecdb_prompt
) -> AsyncIterator[
    Dict[str, Any]
]:
    vecdb = VDBSearchAPI()

    yield {
        "role": "assistant",
        "content": f"Querying vecdb for {query}",
        "gui_role": "tool_use",
    }

    candidates = list(cut_candidates(
        enc=enc,
        candidates=list(vecdb.search(query, 'main', 10)),
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
        "content": prompt(candidates),
        "gui_role": "documents",
        "gui_content": search_result,
        "gui_function": "vecdb",
    }

