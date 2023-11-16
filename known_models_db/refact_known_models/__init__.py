from known_models_db.refact_known_models.utils import ModelSpec
from known_models_db.refact_known_models.utils import ModelRegistry

from known_models_db.refact_known_models.refact import refact_specs
from known_models_db.refact_known_models.huggingface import huggingface_specs

from itertools import chain


models_registry: ModelRegistry = ModelRegistry(chain(refact_specs, huggingface_specs))
