from known_models_db.refact_known_models.utils import ModelSpec
from known_models_db.refact_known_models.utils import ModelRegistry

from known_models_db.refact_known_models.refact import refact_specs
from known_models_db.refact_known_models.huggingface import starcoder_specs
from known_models_db.refact_known_models.huggingface import wizardcoder_specs
from known_models_db.refact_known_models.huggingface import codellama_specs
from known_models_db.refact_known_models.huggingface import deepseek_specs
from known_models_db.refact_known_models.huggingface import starchat_specs
from known_models_db.refact_known_models.huggingface import wizardlm_specs
from known_models_db.refact_known_models.huggingface import llama2_specs

from itertools import chain


models_registry: ModelRegistry = ModelRegistry(chain(
    refact_specs,
    starcoder_specs,
    wizardcoder_specs,
    codellama_specs,
    deepseek_specs,
    starchat_specs,
    wizardlm_specs,
    llama2_specs,
))
