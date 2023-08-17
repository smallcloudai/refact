from typing import List, Union

import openai

__all__ = ['OpenAIEmbeddingSpad']


class OpenAIEmbeddingSpad:
    def __init__(
            self,
            model_name: str = 'text-embedding-ada-002',
            max_tries: int = 3,
    ):
        self._model_name = model_name
        self._max_tries = max_tries

    def create(self, texts: Union[str, List[str]]) -> List[float]:
        texts = texts if isinstance(texts, list) else [texts]
        errors_cnt = 0
        while True:
            try:
                return openai.Embedding.create(
                    input=texts,
                    model=self._model_name,
                )['data'][0]['embedding']
            except Exception as e:
                errors_cnt += 1
                if errors_cnt >= self._max_tries:
                    raise e
