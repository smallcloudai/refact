from .openai_embed_spad import OpenAIEmbeddingSpad
from .gte_embed_spad import GTEEmbeddingSpad

from .embed_spads_utils import ChunkifyFiles


embed_providers = {
    'ada': OpenAIEmbeddingSpad,
    'gte': GTEEmbeddingSpad
}

