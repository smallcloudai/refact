import asyncio

from refact_self_hosting.webgui import selfhost_webutils

from typing import Dict, Any


__all__ = ["Ticket"]


class Ticket:
    def __init__(self, id_prefix):
        self.call: Dict[str, Any] = dict()
        self.call["id"] = id_prefix + selfhost_webutils.random_guid()
        self.cancelled: bool = False
        self.processed_by_infmod_guid: str = ""
        self.streaming_queue = asyncio.queues.Queue()

    def id(self):
        return self.call.get("id", None)

    def done(self):
        if "id" in self.call:
            del self.call["id"]
