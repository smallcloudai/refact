from dataclasses import dataclass
from typing import Optional, Any

from vecdb import VecDB
from encoder import Encoder


@dataclass
class Context:
    db: Optional[VecDB] = None
    encoder: Optional[Encoder] = None
    c_session: Optional[Any] = None
    vecdb_update_required: bool = False


CONTEXT = Context()
