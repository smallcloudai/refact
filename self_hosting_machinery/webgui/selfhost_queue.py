import asyncio
import time
import os
import json
from collections import defaultdict
from self_hosting_machinery import env
from self_hosting_machinery.webgui.selfhost_webutils import log
from fastapi import HTTPException
from typing import Dict, List, Any
import uuid


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

    def __init__(self):
        self._user2gpu_queue: Dict[str, asyncio.Queue] = defaultdict(asyncio.Queue)
        self._models_available: List[str] = []
        self._models_available_ts = 0

    def model_name_to_queue(self, ticket, model_name, no_checks=False):
        available_models = self.models_available()
        if not no_checks and model_name not in available_models:
            log("%s model \"%s\" is not working at this moment" % (ticket.id(), model_name))
            raise HTTPException(status_code=400, detail="model '%s' is not available at this moment." % model_name)
        return self._user2gpu_queue[model_name]

    def models_available(self, force_read: bool = False) -> List[str]:
        t1 = time.time()
        if not force_read and self._models_available_ts + self.CACHE_MODELS_AVAILABLE > t1:
            return self._models_available
        self._models_available = []
        if os.path.exists(env.CONFIG_INFERENCE):
            j = json.load(open(env.CONFIG_INFERENCE, 'r'))
            for model in j["model_assign"]:
                self._models_available.append(model)
            self._models_available_ts = time.time()
            if j.get("openai_api_enable", False):
                self._models_available.append('gpt-3.5-turbo')
                self._models_available.append('gpt-4')
            if j.get("anthropic_api_enable", False):
                self._models_available.append('claude-instant-1.2')
                self._models_available.append('claude-2.1')
        return self._models_available
