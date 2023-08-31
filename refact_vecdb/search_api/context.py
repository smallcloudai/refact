from dataclasses import dataclass, field
from typing import Optional, Any, Dict


__all__ = ['CONTEXT']


@dataclass
class Context:
    vecdb: Optional[Any] = None
    c_sessions: Dict[str, Any] = field(default_factory=dict)
    c_setup_data: Dict[str, Any] = field(default_factory=dict)


CONTEXT = Context()
