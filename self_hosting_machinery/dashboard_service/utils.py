import copy
import pandas as pd

from datetime import datetime
from typing import Dict, Any
from dataclasses import dataclass

from self_hosting_machinery.webgui.selfhost_database import StatisticsService


@dataclass
class StatsDataTables:
    network_df: pd.DataFrame
    robot_human_df: pd.DataFrame
    comp_counters_df: pd.DataFrame
    extra: Dict


class NoDataInDatabase(Exception):
    pass


def network_df_from_ts(stats_service: StatisticsService, ts: int) -> pd.DataFrame:
    prep = stats_service.session.prepare(
        "SELECT * FROM telemetry_network WHERE ts_end >= ? ALLOW FILTERING"
    )

    def fetch_records():
        for record in stats_service.session.execute(
                prep, (ts,)
        ):
            yield {
                "tenant_name": record["tenant_name"],
                "team": record["team"],
                "ts_reported": record["ts_reported"],
                # "ip": record.ip,
                "enduser_client_version": record["enduser_client_version"],
                "counter": record["counter"],
                "error_message": record["error_message"],
                "scope": record["scope"],
                "success": record["success"],
                "url": record["url"],
                "teletype": record["teletype"],
                "ts_start": record["ts_start"],
                "ts_end": record["ts_end"],
            }

    return pd.DataFrame(fetch_records())


def robot_human_df_from_ts(stats_service: StatisticsService, ts: int) -> pd.DataFrame:
    prep = stats_service.session.prepare(
        "SELECT * FROM telemetry_robot_human WHERE ts_end >= ? ALLOW FILTERING"
    )

    def fetch_records():
        for record in stats_service.session.execute(
                prep, (ts,)
        ):
            yield {
                "tenant_name": record["tenant_name"],
                "team": record["team"],
                # "ts_reported": record["ts_reported"],
                # "ip": record.ip,
                # "enduser_client_version": record.enduser_client_version,
                "completions_cnt": record["completions_cnt"],
                "file_extension": record["file_extension"],
                "human_characters": record["human_characters"],
                "model": record["model"],
                "robot_characters": record["robot_characters"],
                # "teletype": record.teletype,
                # "ts_start": record["ts_start"],
                "ts_end": record["ts_end"],
            }

    return pd.DataFrame(fetch_records())


def comp_counters_df_from_ts(stats_service: StatisticsService, ts: int) -> pd.DataFrame:
    prep = stats_service.session.prepare(
        "SELECT * FROM telemetry_comp_counters WHERE ts_end >= ? ALLOW FILTERING"
    )

    def fetch_records():
        for record in stats_service.session.execute(
                prep, (ts,)
        ):
            yield {
                "tenant_name": record["tenant_name"],
                "team": record["team"],
                "ts_reported": record["ts_reported"],
                # "ip": record.ip,
                "enduser_client_version": record["enduser_client_version"],
                # "counters_json_text": record.counters_json_text,
                "file_extension": record["file_extension"],
                "model": record["model"],
                "multiline": record["multiline"],
                # "teletype": record.teletype,
                "ts_start": record["ts_start"],
                "ts_end": record["ts_end"],
            }

    return pd.DataFrame(fetch_records())


def retrieve_all_data_tables(stats_service: StatisticsService) -> StatsDataTables:
    current_year = datetime.now().year
    start_of_year = datetime(current_year, 1, 1, 0, 0, 0, 0)
    timestamp_start_of_year = int(start_of_year.timestamp())

    extra = {}
    network_df = network_df_from_ts(stats_service, timestamp_start_of_year)
    robot_human_df = robot_human_df_from_ts(stats_service, timestamp_start_of_year)
    comp_counters_df = comp_counters_df_from_ts(stats_service, timestamp_start_of_year)

    if network_df.empty or robot_human_df.empty or comp_counters_df.empty:
        raise NoDataInDatabase("No data in database!")

    network_df['dt_end'] = pd.to_datetime(network_df['ts_end'], unit='s')
    robot_human_df['dt_end'] = pd.to_datetime(robot_human_df['ts_end'], unit='s')
    comp_counters_df['dt_end'] = pd.to_datetime(comp_counters_df['ts_end'], unit='s')

    network_df.sort_values(by='dt_end', inplace=True)
    robot_human_df.sort_values(by='dt_end', inplace=True)
    comp_counters_df.sort_values(by='dt_end', inplace=True)

    extra["week_n_to_fmt"] = {
        week_n: datetime.strftime(group["dt_end"].iloc[0], "%b %d")
        for week_n, group in network_df.groupby(network_df['dt_end'].dt.isocalendar().week)
    }
    extra["day_to_fmt"] = [
        datetime.strftime(group["dt_end"].iloc[0], "%b %d")
        for date, group in network_df.groupby(network_df['dt_end'].dt.date)
    ]
    extra["month_to_fmt"] = {
        month_n: datetime.strftime(group["dt_end"].iloc[0], "%b")
        for month_n, group in network_df.groupby(network_df['dt_end'].dt.month)
    }

    return StatsDataTables(
        network_df=network_df,
        robot_human_df=robot_human_df,
        comp_counters_df=comp_counters_df,
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
