from refact_vecdb.embeds_api.context import CONTEXT as C
from refact_vecdb.embeds_api.embed_spads import embed_providers
from refact_vecdb.embeds_api.model import VDBTextEncoderProcess

__all__ = ['bootstrap']


def setup_models():
    for provider in embed_providers.keys():
        enc_params = dict(provider=provider)
        C.models[f'{provider}_index'] = VDBTextEncoderProcess(enc_params)
        C.models[f'{provider}_search'] = VDBTextEncoderProcess(enc_params)


def bootstrap():
    setup_models()
