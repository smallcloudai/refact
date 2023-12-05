import time

from typing import Dict, Any, Optional

import pandas as pd

from fastapi import APIRouter
from fastapi.responses import JSONResponse

from self_hosting_machinery.dashboard_service.utils import StatsDataTables
from self_hosting_machinery.webgui.selfhost_statistics import DashTeamsGenDashData
from self_hosting_machinery.dashboard_service.dashboards.dash_prime import barplot_completions, barplot_rh


def teams_data(rh_df) -> Dict[str, Any]:
    res = {}
    for team, group in rh_df.groupby(rh_df["team"]):
        res[team] = {
            "users": list(group["tenant_name"].unique()),
        }
    return res


def barplot_completions_users(
        rh_df: pd.DataFrame,
        extra: Dict,
        post: DashTeamsGenDashData,
) -> Dict[str, Any]:
    res = {}
    users_selected = post.users_selected

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


class DashboardTeamsRouter(APIRouter):
    def __init__(
            self,
            data_tables: Optional[StatsDataTables] = None,
            *args, **kwargs
    ):
        super().__init__(*args, **kwargs)
        self._data_tables = data_tables
        self.add_api_route("/plots-data", self._plots_data, methods=["GET"])
        self.add_api_route("/plots-data", self._generate_dashboard, methods=["POST"])

    async def _plots_data(self):
        if self._data_tables is None:
            return JSONResponse(
                content={"error": "users sent no statistics so far"},
                media_type='application/json',
                status_code=404,
            )
        time_start = time.time()
        data = {
            "teams_data": teams_data(self._data_tables.robot_human_df),
        }
        print(f"DashboardTeamsRouter._plots_data took: {round(time.time() - time_start, 3)}s")
        return JSONResponse(
            content=data,
            media_type='application/json'
        )

    async def _generate_dashboard(self, post: DashTeamsGenDashData):
        if self._data_tables is None:
            return JSONResponse(
                content={"error": "users sent no statistics so far"},
                media_type='application/json',
                status_code=404,
            )
        time_start = time.time()
        data = {
            "barplot_completions_users": barplot_completions_users(
                self._data_tables.robot_human_df,
                self._data_tables.extra,
                post,
            ),
        }
        print(f"DashboardTeamsRouter._generate_dashboard took: {round(time.time() - time_start, 3)}s")
        return JSONResponse(
            content=data,
            media_type='application/json'
        )
