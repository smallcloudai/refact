from dataclasses import dataclass, field
from typing import Optional, Dict, Any
from multiprocessing import Manager


__all__ = ['CONTEXT']


@dataclass
class Context:
    q_manager: Optional[Manager] = None
    models: Dict[str, Any] = field(default_factory=dict)


CONTEXT = Context()


