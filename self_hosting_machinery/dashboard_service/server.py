import argparse

from argparse import ArgumentParser

import uvicorn

from fastapi import FastAPI

from self_hosting_machinery.webgui.selfhost_database import RefactDatabase
from self_hosting_machinery.webgui.selfhost_database import StatisticsService
from self_hosting_machinery.dashboard_service.dashboards.dash_prime import DashboardPrimeRouter
from self_hosting_machinery.dashboard_service.dashboards.dash_teams import DashboardTeamsRouter
from self_hosting_machinery.dashboard_service.utils import retrieve_all_data_tables


def main(args: argparse.Namespace):
    app = FastAPI()
    db = RefactDatabase()
    stats_service = StatisticsService(db)

    # data is fetched once on a start, but service is being restarted every 24h by watchdog
    data_tables = retrieve_all_data_tables(stats_service)
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
    uvicorn.run(app, host=args.host, port=args.port, loop="uvloop", timeout_keep_alive=600)


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", default=8010, type=int)
    args = parser.parse_args()
    main(args)
