from known_models_db.refact_known_models.refact import refact_mini_db
from known_models_db.refact_known_models.huggingface import huggingface_mini_db
from known_models_db.refact_known_models.passthrough import passthrough_mini_db

models_mini_db = {**refact_mini_db, **huggingface_mini_db}
