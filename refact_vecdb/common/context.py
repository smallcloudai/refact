from dataclasses import dataclass, field
from typing import Any, Dict


__all__ = ['CONTEXT']


@dataclass
class Context:
    c_session: Any = None
    c_models: Dict[str, Any] = field(default_factory=dict)
    c_setup_data: Dict[str, Any] = field(default_factory=dict)
    vecdb: Dict[str, Any] = field(default_factory=dict)
    processes: Dict[str, Any] = field(default_factory=dict)


CONTEXT = Context()
