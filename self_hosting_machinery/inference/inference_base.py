import importlib

from refact_scratchpads_no_gpu.stream_results import UploadProxy

from typing import Dict, Any


def modload(import_str):
    import_mod, import_class = import_str.rsplit(":", 1)
    model = importlib.import_module(import_mod)
    Class = getattr(model, import_class, None)
    if Class is None:
        raise ValueError("cannot find \"%s\"" % import_str)
    return Class


class InferenceBase:

    def infer(self, request: Dict[str, Any], upload_proxy: UploadProxy, upload_proxy_args: Dict):
        raise NotImplementedError()
