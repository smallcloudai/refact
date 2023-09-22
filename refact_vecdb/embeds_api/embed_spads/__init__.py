from .openai_embed_spad import OpenAIEmbeddingSpad
from .gte_embed_spad import GTEEmbeddingSpad
from .embed_spads_utils import ChunkifyFiles


__all__ = [
    'OpenAIEmbeddingSpad',
    'GTEEmbeddingSpad',
    'ChunkifyFiles',
    'embed_providers',
    'models'
]


embed_providers = {
    'ada': OpenAIEmbeddingSpad,
    'gte': GTEEmbeddingSpad
}


models = list(embed_providers.keys())
