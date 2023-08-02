import openai


class OpenAIEmbedding:
    def __init__(
            self,
            model: str,
            max_tries: int = 3,
    ):
        self._model = model
        self._max_tries = max_tries

    def create(self, text: str):
        errors_cnt = 0
        while True:
            try:
                return openai.Embedding.create(
                    input=[text],
                    model=self._model,
                )['data'][0]['embedding']
            except Exception as e:
                errors_cnt += 1
                if errors_cnt >= self._max_tries:
                    raise e
