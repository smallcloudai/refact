from dataclasses import dataclass, field
from typing import Any, Dict


__all__ = ['CONTEXT']


@dataclass
class Context:
    c_sessions: Dict[str, Any] = field(default_factory=dict)
    c_setup_data: Dict[str, Any] = field(default_factory=dict)


CONTEXT = Context()
