from fastapi import Header
from fastapi import HTTPException

from refact_webgui.webgui.selfhost_fastapi_completions import NlpCompletion
from refact_webgui.webgui.selfhost_fastapi_completions import EmbeddingsStyleOpenAI
from refact_webgui.webgui.selfhost_fastapi_completions import CompletionsRouter


__all__ = ["ProxyCompletionsRouter"]


class ProxyCompletionsRouter(CompletionsRouter):

    async def _completions(self, post: NlpCompletion, authorization: str = Header(None)):
        raise HTTPException(status_code=400, detail="completions handler is not available for proxy")

    async def _embeddings_style_openai(self, post: EmbeddingsStyleOpenAI, authorization: str = Header(None)):
        raise HTTPException(status_code=400, detail="embeddings handler is not available for proxy")
