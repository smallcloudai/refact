from datetime import datetime
from typing import Dict, List, Any
import pandas as pd

from refact_webgui.dashboards.utils import complete_date_axis, StatsDataFrames


def robot_human_ratio(robot: int, human: int) -> float:
    if human == 0:
        return 1
    if robot == 0:
        return 0
    # in older versions of refact LSP negative values of human metric existed
    if robot + human == 0:
        return 0
    return round(robot / (robot + human), 2)


def barplot_rh(
        rh_df: pd.DataFrame,
        extra: Dict
) -> Dict:
    def create_chart_data(data_dict, x_values, title, date_kind):
        return {
            "data": {key: [v[key] for v in data_dict.values()] for key in ["robot", "human", "ratio", "completions"]},
            "x_axis": list(x_values),
            "x_axis_type": "category",
            "title": title,
            "date_kind": date_kind,
        }

    res = {}
    day_to_rh = {
        datetime.strftime(group["dt_end"].iloc[0], "%b %d"): {
            "robot": (robot := int(group["robot_characters"].sum())),
            "human": (human := int(group["human_characters"].sum())),
            "ratio": robot_human_ratio(robot, human),
            "completions": int(group["completions_cnt"].sum()),
        } for date, group in rh_df.groupby(rh_df['dt_end'].dt.date)
    }
    week_to_rh = {
        group["dt_end"].iloc[0].week: {
            "robot": (robot := int(group["robot_characters"].sum())),
            "human": (human := int(group["human_characters"].sum())),
            "ratio": robot_human_ratio(robot, human),
            "completions": int(group["completions_cnt"].sum()),
        } for date, group in rh_df.groupby(rh_df['dt_end'].dt.isocalendar().week)
    }
    month_to_rh = {
        group["dt_end"].iloc[0].month: {
            "robot": (robot := int(group["robot_characters"].sum())),
            "human": (human := int(group["human_characters"].sum())),
            "ratio": robot_human_ratio(robot, human),
            "completions": int(group["completions_cnt"].sum()),
        } for date, group in rh_df.groupby(rh_df['dt_end'].dt.month)
    }

    default_val = {"robot": 0, "human": 0, "ratio": 0, "completions": 0}
    day_to_rh = complete_date_axis(day_to_rh, default_val, "daily", extra)
    week_to_rh_fmt = complete_date_axis(week_to_rh, default_val, "weekly", extra)
    month_to_rh_fmt = complete_date_axis(month_to_rh, default_val, "monthly", extra)

    res["daily"] = create_chart_data(day_to_rh, day_to_rh.keys(), "Assistant vs Human Daily", "daily")
    res["weekly"] = create_chart_data(week_to_rh_fmt, week_to_rh_fmt.keys(), "Assistant vs Human Weekly", "weekly")
    res["monthly"] = create_chart_data(month_to_rh_fmt, month_to_rh_fmt.keys(), "Assistant vs Human Monthly", "monthly")

    res["btns_data"] = {
        "btns_text": ["daily", "weekly", "monthly"],
        "default": "weekly",
    }
    return res


def barplot_completions(
        rh_df: pd.DataFrame,
        extra: Dict
):
    def create_chart_data(data_dict, x_values, title, date_kind):
        return {
            "data": {key: [v[key] for v in data_dict.values()] for key in ["completions"]},
            "x_axis": list(x_values),
            "x_axis_type": "category",
            "title": title,
            "date_kind": date_kind,
        }
    res = {}
    day_to_comp_cnt = {
        datetime.strftime(group["dt_end"].iloc[0], "%b %d"): {"completions": int(group["completions_cnt"].sum())}
        for date, group in rh_df.groupby(rh_df['dt_end'].dt.date)
    }
    week_to_comp_cnt = {
        group["dt_end"].iloc[0].week: {"completions": int(group["completions_cnt"].sum())}
        for date, group in rh_df.groupby(rh_df['dt_end'].dt.isocalendar().week)
    }
    month_to_comp_cnt = {
        group["dt_end"].iloc[0].month: {"completions": int(group["completions_cnt"].sum())}
        for date, group in rh_df.groupby(rh_df['dt_end'].dt.month)
    }

    default_val = {"completions": 0}
    day_to_comp_cnt = complete_date_axis(day_to_comp_cnt, default_val, "daily", extra)
    week_to_comp_cnt_fmt = complete_date_axis(week_to_comp_cnt, default_val, "weekly", extra)
    month_to_comp_cnt_fmt = complete_date_axis(month_to_comp_cnt, default_val, "monthly", extra)

    res["daily"] = create_chart_data(day_to_comp_cnt, day_to_comp_cnt.keys(), "Completions Daily", "daily")
    res["weekly"] = create_chart_data(week_to_comp_cnt_fmt, week_to_comp_cnt_fmt.keys(), "Completions Weekly", "weekly")
    res["monthly"] = create_chart_data(month_to_comp_cnt_fmt, month_to_comp_cnt_fmt.keys(), "Completions Monthly", "monthly")
    res["btns_data"] = {
        "btns_text": ["daily", "weekly", "monthly"],
        "default": "daily",
    }
    return res


def barplot_users(
        rh_df: pd.DataFrame,
        extra: Dict
):
    def create_chart_data(data_dict, x_values, title, date_kind):
        return {
            "data": {key: [v[key] for v in data_dict.values()] for key in ["users"]},
            "x_axis": list(x_values),
            "x_axis_type": "category",
            "title": title,
            "date_kind": date_kind,
        }

    res = {}
    day_to_users_cnt = {
        datetime.strftime(group["dt_end"].iloc[0], "%b %d"): {"users": int(group["tenant_name"].nunique())}
        for date, group in rh_df.groupby(rh_df['dt_end'].dt.date)
    }
    week_to_users_cnt = {
        group["dt_end"].iloc[0].week: {"users": int(group["tenant_name"].nunique())}
        for date, group in rh_df.groupby(rh_df['dt_end'].dt.isocalendar().week)
    }
    month_to_users_cnt = {
        group["dt_end"].iloc[0].month: {"users": int(group["tenant_name"].nunique())}
        for date, group in rh_df.groupby(rh_df['dt_end'].dt.month)
    }

    default_val = {"users": 0}
    day_to_users_cnt = complete_date_axis(day_to_users_cnt, default_val, "daily", extra)
    week_to_users_cnt_fmt = complete_date_axis(week_to_users_cnt, default_val, "weekly", extra)
    month_to_users_cnt_fmt = complete_date_axis(month_to_users_cnt, default_val, "monthly", extra)

    res["daily"] = create_chart_data(day_to_users_cnt, day_to_users_cnt.keys(), "Users Daily", "daily")
    res["weekly"] = create_chart_data(week_to_users_cnt_fmt, week_to_users_cnt_fmt.keys(), "Users Weekly", "weekly")
    res["monthly"] = create_chart_data(month_to_users_cnt_fmt, month_to_users_cnt_fmt.keys(), "Users Monthly", "monthly")

    res["btns_data"] = {
        "btns_text": ["daily", "weekly", "monthly"],
        "default": "daily",
    }
    return res


def table_lang_comp_stats(rh_df: pd.DataFrame):
    languages = [e for e in rh_df["file_extension"].unique() if e.startswith(".")]

    def format_row(row: List[Any]):
        # TODO(valerii): move to js
        new_row = []
        for e in row:
            if isinstance(e, float) or isinstance(e, int):
                if e // 1_000_000:
                    new_row.append(f"{round(e / 1_000_000, 2)}M")
                elif e // 1_000:
                    new_row.append(f"{round(e / 1_000, 2)}k")
                else:
                    new_row.append(e)
            else:
                new_row.append(e)
        return new_row

    def extract_stats(df: pd.DataFrame, date_kind: str) -> Dict:
        res_loc = {}
        for lang, group in df.groupby("file_extension"):
            if lang not in languages:
                continue
            res_loc[lang] = {
                "Refact": (robot := int(group["robot_characters"].sum())),
                "Human": (human := int(group["human_characters"].sum())),
                "Total (characters)": robot + human,
                "Refact Impact": robot_human_ratio(robot, human),
                "Completions": int(group["completions_cnt"].sum()),
                "Users": int(group["tenant_name"].nunique()),
            }
        # into row-like fmt
        sorted_vals: List[List] = sorted([[k, *v.values()] for k, v in res_loc.items()], key=lambda x: x[3], reverse=True)
        fmt_vals = [format_row(row) for row in sorted_vals]
        res_loc = {
            'data': fmt_vals,
            'columns': ['Language', *res_loc[list(res_loc.keys())[0]].keys()],
            'title': f"Refact's impact by language: {date_kind}"
        }
        return res_loc

    res = {
        "last week": extract_stats(
            rh_df.loc[rh_df["dt_end"].dt.isocalendar().week == rh_df["dt_end"].dt.isocalendar().week.max()],
            "last week"
        ),
        "last month": extract_stats(
            rh_df.loc[rh_df["dt_end"].dt.month == rh_df["dt_end"].dt.month.max()],
            "last month"
        ),
        "all time": extract_stats(
            rh_df,
            "all time"
        ),
        "btns_data": {
            "btns_text": ["last week", "last month", "all time"],
            "default": "all time",
        }
    }

    return res


def dashboard_prime(data_tables: StatsDataFrames):
    return {
        "table_lang_comp_stats": table_lang_comp_stats(data_tables.robot_human_df),
        "barplot_rh": barplot_rh(data_tables.robot_human_df, data_tables.extra),
        "barplot_completions": barplot_completions(data_tables.robot_human_df, data_tables.extra),
        "barplot_users": barplot_users(data_tables.robot_human_df, data_tables.extra),
    }
