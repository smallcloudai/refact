from typing import Dict, Any, List

import pandas as pd

from self_hosting_machinery.dashboards.utils import StatsDataFrames
from self_hosting_machinery.dashboards.dash_prime import barplot_completions, barplot_rh


def get_teams_data(rh_df) -> Dict[str, Any]:
    res = {}
    for team, group in rh_df.groupby(rh_df["team"]):
        res[team] = {
            "users": list(group["tenant_name"].unique()),
        }
    return res


def barplot_completions_users(
        rh_df: pd.DataFrame,
        extra: Dict,
        users_selected: List,
) -> Dict[str, Any]:
    res = {}

    rh_df = rh_df.loc[rh_df["tenant_name"].isin(users_selected)]

    user_series = {
        tenant_name: barplot_completions(rh_df.loc[rh_df["tenant_name"] == tenant_name], extra)
        for tenant_name in users_selected
    }
    rh_tables = {"daily": {}, "weekly": {}, "monthly": {}}
    for tenant_name in users_selected:
        filt_df = rh_df.loc[rh_df["tenant_name"] == tenant_name]
        tenant_rh = barplot_rh(filt_df, extra)

        for date_kind in ["daily", "weekly", "monthly"]:
            keys, key_vals = list(tenant_rh[date_kind]['data'].keys()), list(tenant_rh[date_kind]['data'].values())
            for i in range(len(key_vals[0])):
                vals = [v[i] for v in key_vals]
                vals_insert = [tenant_name, vals[0], vals[1], vals[0] + vals[1], vals[2], vals[3]]
                rh_tables[date_kind].setdefault(tenant_rh[date_kind]['x_axis'][i], []).append(vals_insert)

    rh_tables_cols = ['User', 'Assistant', 'Human', 'Total', 'Ratio', 'Completions']

    for date_type in ["daily", "weekly", "monthly"]:
        res[date_type] = {
            "data": {k: v[date_type]["data"]["completions"] for k, v in user_series.items()},
            "x_axis": [v[date_type]["x_axis"] for k, v in user_series.items()][0],
            "title": [v[date_type]["title"] for k, v in user_series.items()][0],
            "date_kind": [v[date_type]["date_kind"] for k, v in user_series.items()][0],
            "table_data": rh_tables[date_type],
            "table_cols": rh_tables_cols,
        }

    res["btns_data"] = {
        "btns_text": ["daily", "weekly", "monthly"],
        "default": "daily",
    }
    return res


def teams_data(data_tables: StatsDataFrames) -> Dict:
    return {
        "teams_data": get_teams_data(data_tables.robot_human_df),
    }


def dashboard_teams(
        data_tables: StatsDataFrames,
        users_selected,
) -> Dict:
    return {
        "barplot_completions_users": barplot_completions_users(
            data_tables.robot_human_df,
            data_tables.extra,
            users_selected,
        ),
    }
