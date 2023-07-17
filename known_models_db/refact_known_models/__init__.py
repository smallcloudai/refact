from known_models_db.refact_known_models.refact import refact_mini_db
from known_models_db.refact_known_models.bigcode import big_code_mini_db
from known_models_db.refact_known_models.huggingface_gptq import huggingface_gptq_mini_db
models_mini_db = {**refact_mini_db, **big_code_mini_db, **huggingface_gptq_mini_db}
