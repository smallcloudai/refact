from dataclasses import dataclass, field
from typing import Optional, Any, Dict, List


@dataclass
class Context:
    vecdb: Optional[Any] = None
    provider: Optional[str] = None
    c_session: Optional[Any] = None

    vecdb_update_required: bool = False

    encoder: Optional[Any] = None
    models: Dict[str, Any] = field(default_factory=dict)
    tokenizers: Dict[str, Any] = field(default_factory=dict)
    status_ongoing: Dict[str, Dict] = field(default_factory=dict)


CONTEXT = Context()
