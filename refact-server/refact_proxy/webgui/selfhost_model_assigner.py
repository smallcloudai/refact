from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner

from typing import Dict, Any


__all__ = ["ProxyModelAssigner"]


class ProxyModelAssigner(ModelAssigner):

    @property
    def models_db(self) -> Dict[str, Any]:
        return {}

    @property
    def models_info(self):
        return {"models": []}

    @property
    def model_assignment(self):
        return {"model_assign": {}}

    def config_inference_mtime(self) -> int:
        return 0

    def to_completion_model_record(self, model_name: str, model_info: Dict[str, Any]) -> Dict[str, Any]:
        raise NotImplementedError()

    def to_chat_model_record(self, model_name: str, model_info: Dict[str, Any]) -> Dict[str, Any]:
        raise NotImplementedError()

    def models_to_watchdog_configs(self, inference_config=None):
        raise NotImplementedError()

    @staticmethod
    def has_available_weights(model_path: str) -> bool:
        raise NotImplementedError()

    @property
    def _model_cfg_template(self) -> Dict:
        raise NotImplementedError()

    def _has_loras(self, model_name: str) -> bool:
        raise NotImplementedError()

    def first_run(self):
        raise NotImplementedError()

    @property
    def devices(self):
        raise NotImplementedError()

    def _model_inference_setup(self, inference_config: Dict[str, Any]) -> Dict[str, Any]:
        raise NotImplementedError()
