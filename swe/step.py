from typing import Set, Any


__all__ = ["Step"]


class Step:
    def __init__(
            self,
            base_url: str,
            model_name: str,
            temperature: float = 0.2,
            max_depth: int = 8):
        self._base_url = base_url
        self._model_name = model_name
        self._temperature = temperature
        self._max_depth = max_depth

    @property
    def _tools(self) -> Set[str]:
        raise NotImplementedError()

    async def process(self, **kwargs) -> Any:
        raise NotImplementedError()
