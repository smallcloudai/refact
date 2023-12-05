import argparse
import time

from argparse import ArgumentParser

import uvicorn

from fastapi import FastAPI

from self_hosting_machinery.webgui.selfhost_database import RefactDatabase
from self_hosting_machinery.webgui.selfhost_database import StatisticsService
from self_hosting_machinery.dashboard_service.dashboards.dash_prime import DashboardPrimeRouter
from self_hosting_machinery.dashboard_service.dashboards.dash_teams import DashboardTeamsRouter
from self_hosting_machinery.dashboard_service.utils import retrieve_all_data_tables, NoDataInDatabase


def create_app_with_routers(data_tables):
    app = FastAPI()
    app.include_router(
        DashboardPrimeRouter(
            prefix="/dash-prime",
            tags=["dashboards"],
            data_tables=data_tables
        )
    )
    app.include_router(
        DashboardTeamsRouter(
            prefix="/dash-teams",
            tags=["dashboards"],
            data_tables=data_tables
        )
    )
    return app


def main(args: argparse.Namespace):
    db = RefactDatabase()
    stats_service = StatisticsService(db)

    # data is fetched once on a start, but service is being restarted every 24h by watchdog
    try:
        data_tables = retrieve_all_data_tables(stats_service)
    except NoDataInDatabase:
        data_tables = None

    app = create_app_with_routers(data_tables)
    uvicorn.run(app, host=args.host, port=args.port, loop="uvloop", timeout_keep_alive=600)


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", default=8010, type=int)
    args = parser.parse_args()
    main(args)
