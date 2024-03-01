import copy
import pandas as pd

from dataclasses import dataclass
from datetime import datetime
from typing import Dict, Any


@dataclass
class StatsDataFrames:
    robot_human_df: pd.DataFrame
    extra: Dict


# has to be ignored as they send deviated statistics
IGNORE_PLUGIN_VERSION = [
    "jetbrains-1.2.16",
    "jetbrains-1.2.15",
    "jetbrains-1.2.13",
    "jetbrains-1.2.14",
    "refact-2.0.125",
    "refact-2.0.124",
    "refact-2.0.123",
]


def complete_date_axis(
        data: Dict[Any, Any],
        default_val: Any,
        date_type: str,
        extra: Dict
) -> Dict[Any, Any]:
    data_fmt = {}
    if date_type == "daily":
        for day_fmt in extra["day_to_fmt"]:
            data.setdefault(day_fmt, copy.deepcopy(default_val))
        data_fmt = dict(sorted(data.items(), key=lambda x: datetime.strptime(x[0], "%b %d, %y")))
        return data_fmt

    elif date_type == "weekly":
        for week_n, week_fmt in extra["week_n_to_fmt"].items():
            data_fmt.setdefault(week_fmt, copy.deepcopy(default_val))
            data_fmt[week_fmt].update(data.get(week_n, {}))
        data_fmt = dict(sorted(data_fmt.items(), key=lambda x: datetime.strptime(x[0], "%b %d, %y")))
        return data_fmt

    elif date_type == "monthly":
        for month_n, month_fmt in extra["month_to_fmt"].items():
            data_fmt.setdefault(month_fmt, copy.deepcopy(default_val))
            data_fmt[month_fmt].update(data.get(month_n, {}))
        data_fmt = dict(sorted(data_fmt.items(), key=lambda x: datetime.strptime(x[0], "%b")))
        return data_fmt

    else:
        raise ValueError(f"date_type: {date_type} is not implemented!")
