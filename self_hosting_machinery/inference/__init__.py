import logging

from functools import partial

from self_hosting_machinery import NOTICE

from self_hosting_machinery.inference.inference_base import modload
from self_hosting_machinery.inference.inference_base import InferenceBase
from self_hosting_machinery.inference.inference_hf import InferenceHF
from self_hosting_machinery.inference.inference_embeddings import InferenceEmbeddings


logger = logging.getLogger("MODEL")
log = partial(logger.log, NOTICE)
