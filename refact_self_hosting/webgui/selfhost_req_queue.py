from collections import defaultdict, deque
from typing import Dict, List, Optional, Any
import asyncio
import weakref
from refact_self_hosting.webgui import selfhost_webutils



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


global_stats = defaultdict(int)  # set of keys is finite


global_user2gpu_queue = defaultdict(asyncio.Queue)   # for each model there is a queue


global_id2ticket: Dict[str, Ticket] = weakref.WeakValueDictionary()

