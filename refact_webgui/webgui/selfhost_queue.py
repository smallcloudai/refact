import asyncio
import time
import os
import json
from collections import defaultdict
from fastapi import HTTPException
from typing import Dict, List, Any, Tuple
import uuid

from refact_utils.scripts import env
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from refact_webgui.webgui.selfhost_webutils import log


class Ticket:
    def __init__(self, id_prefix):
        self.call: Dict[str, Any] = dict()
        random_guid = str(uuid.uuid4()).replace("-", "")[0:12]
        self.call["id"] = id_prefix + random_guid
        self.cancelled: bool = False
        self.processed_by_infmod_guid: str = ""
        self.streaming_queue = asyncio.queues.Queue()

    def id(self):
        return self.call.get("id", None)

    def done(self):
        if "id" in self.call:
            del self.call["id"]


class InferenceQueue:
    CACHE_MODELS_AVAILABLE = 5

    def __init__(self, model_assigner: ModelAssigner):
        self._user2gpu_queue: Dict[str, asyncio.Queue] = defaultdict(asyncio.Queue)
        self._models_available: List[str] = []
        self._models_available_ts = 0
        self._model_assigner = model_assigner

    def model_name_to_queue(self, ticket, model_name, no_checks=False):
        available_models = self.models_available()
        if not no_checks and model_name not in available_models:
            log("%s model \"%s\" is not working at this moment" % (ticket.id(), model_name))
            raise HTTPException(status_code=400, detail="model '%s' is not available at this moment." % model_name)
        return self._user2gpu_queue[model_name]

    def models_available(self, force_read: bool = False) -> List[str]:

        def _add_models_for_passthrough_provider(provider):
            self._models_available.extend(k for k, v in self._model_assigner.passthrough_mini_db.items() if v.get('provider') == provider)

        t1 = time.time()
        if not force_read and self._models_available_ts + self.CACHE_MODELS_AVAILABLE > t1:
            return self._models_available
        self._models_available = []
        if os.path.exists(env.CONFIG_INFERENCE):
            j = json.load(open(env.CONFIG_INFERENCE, 'r'))
            for model in j["model_assign"]:
                self._models_available.append(model)
            self._models_available_ts = time.time()

            if j.get("openai_api_enable"):
                _add_models_for_passthrough_provider('openai')
            if j.get("anthropic_api_enable"):
                _add_models_for_passthrough_provider('anthropic')

        return self._models_available

    def completion_model(self) -> Tuple[str, str]:

        if os.path.exists(env.CONFIG_INFERENCE):
            j = json.load(open(env.CONFIG_INFERENCE, 'r'))
            for model in j["model_assign"]:
                if "completion" in self._model_assigner.models_db.get(model, {}).get("filter_caps", {}):
                    return model, ""

        return "", f"completion model is not set"
