import copy

import pandas as pd

from datetime import datetime
from typing import Dict, Any
from dataclasses import dataclass


from self_hosting_machinery.webgui.selfhost_database import StatisticsService


@dataclass
class StatsDataFrames:
    robot_human_df: pd.DataFrame
    extra: Dict


async def compose_data_frames(
        stats_service: StatisticsService
) -> StatsDataFrames:
    current_year = datetime.now().year
    start_of_year = datetime(current_year, 1, 1, 0, 0, 0, 0)
    timestamp_start_of_year = int(start_of_year.timestamp())

    user_to_team_dict = await stats_service.select_users_to_team()
    rh_records = await stats_service.select_rh_from_ts(timestamp_start_of_year)

    robot_human_df = pd.DataFrame(rh_records)
    robot_human_df['dt_end'] = pd.to_datetime(robot_human_df['ts_end'], unit='s')
    robot_human_df['team'] = robot_human_df['tenant_name'].map(lambda x: user_to_team_dict.get(x, "unassigned"))
    robot_human_df.sort_values(by='dt_end', inplace=True)

    extra = {"week_n_to_fmt": {
        week_n: datetime.strftime(group["dt_end"].iloc[0], "%b %d")
        for week_n, group in robot_human_df.groupby(robot_human_df['dt_end'].dt.isocalendar().week)
    }, "day_to_fmt": [
        datetime.strftime(group["dt_end"].iloc[0], "%b %d")
        for date, group in robot_human_df.groupby(robot_human_df['dt_end'].dt.date)
    ], "month_to_fmt": {
        month_n: datetime.strftime(group["dt_end"].iloc[0], "%b")
        for month_n, group in robot_human_df.groupby(robot_human_df['dt_end'].dt.month)
    }}

    return StatsDataFrames(
        robot_human_df=robot_human_df,
        extra=extra,
    )


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
        data_fmt = dict(sorted(data.items(), key=lambda x: datetime.strptime(x[0], "%b %d")))
        return data_fmt

    elif date_type == "weekly":
        for week_n, week_fmt in extra["week_n_to_fmt"].items():
            data_fmt.setdefault(week_fmt, copy.deepcopy(default_val))
            data_fmt[week_fmt].update(data.get(week_n, {}))
        data_fmt = dict(sorted(data_fmt.items(), key=lambda x: datetime.strptime(x[0], "%b %d")))
        return data_fmt

    elif date_type == "monthly":
        for month_n, month_fmt in extra["month_to_fmt"].items():
            data_fmt.setdefault(month_fmt, copy.deepcopy(default_val))
            data_fmt[month_fmt].update(data.get(month_n, {}))
        data_fmt = dict(sorted(data_fmt.items(), key=lambda x: datetime.strptime(x[0], "%b")))
        return data_fmt

    else:
        raise ValueError(f"date_type: {date_type} is not implemented!")
