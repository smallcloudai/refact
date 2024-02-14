import json

from typing import List, Any, Dict

from more_itertools import chunked
from pydantic import BaseModel
from fastapi.responses import StreamingResponse
from fastapi import APIRouter, Request, HTTPException, Header
from fastapi.responses import JSONResponse

from self_hosting_machinery.dashboards.dash_prime import dashboard_prime
from self_hosting_machinery.dashboards.dash_teams import teams_data, dashboard_teams
from self_hosting_machinery.webgui.selfhost_database import StatisticsService, ScyllaBatchInserter
from self_hosting_machinery.webgui.selfhost_login import RefactSession
from self_hosting_machinery.dashboards.utils import compose_data_frames


__all__ = ["BaseTabStatisticsRouter", "TabStatisticsRouter", "DashTeamsGenDashData"]


class TelemetryBasicData(BaseModel):
    enduser_client_version: str
    records: List[Any]
    teletype: str
    ts_start: int
    ts_end: int

    def clamp(self) -> Dict[str, Any]:
        return {
            "enduser_client_version": self.enduser_client_version,
            "records": self.records,
            "teletype": self.teletype,
            "ts_start": self.ts_start,
            "ts_end": self.ts_end,
        }


class DashTeamsGenDashData(BaseModel):
    users_selected: List[str]


class BaseTabStatisticsRouter(APIRouter):

    def __init__(
            self,
            stats_service: StatisticsService,
            *args, **kwargs
    ):
        super().__init__(*args, **kwargs)
        self._stats_service = stats_service
        self._stats_service_not_available_response = JSONResponse(
            content={
                'error': "Statistics service is not ready, waiting for database connection",
            },
            media_type='application/json',
            status_code=500)
        self.add_api_route('/rh-stats', self._rh_stats, methods=["GET"])
        self.add_api_route("/telemetry-basic", self._telemetry_basic, methods=["POST"])
        self.add_api_route("/telemetry-snippets", self._telemetry_snippets, methods=["POST"])
        self.add_api_route('/dash-prime', self._dash_prime_get, methods=['GET'])
        self.add_api_route('/dash-teams', self._dash_teams_get, methods=['GET'])
        self.add_api_route('/dash-teams', self._dash_teams_post, methods=['POST'])

    def _account_from_bearer(self, authorization: str) -> str:
        raise NotImplementedError()

    async def _rh_stats(self, authorization: str = Header(None)):
        account = self._account_from_bearer(authorization)
        if not self._stats_service.is_ready:
            raise HTTPException(status_code=500, detail="Statistics service is not ready, waiting for database connection")

        async def streamer():
            records = []
            async for r in self._stats_service.get_robot_human_for_account(account):
                records.append(r)

            for records_batch in chunked(records, 100):
                yield json.dumps({
                    "retcode": "OK",
                    "data": records_batch
                }) + '\n'

        try:
            return StreamingResponse(streamer(), media_type='text/event-stream')
        except Exception as e:
            raise HTTPException(status_code=500, detail=str(e))

    async def _dash_prime_get(self):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        data_tables = await compose_data_frames(self._stats_service)

        if not data_tables or data_tables.robot_human_df.empty or not data_tables.extra:
            return JSONResponse(
                content={"error": "users sent no statistics so far"},
                media_type='application/json',
                status_code=404,
            )

        data = dashboard_prime(data_tables)
        return JSONResponse(
            content=data,
            media_type='application/json'
        )

    async def _dash_teams_get(self):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        data_tables = await compose_data_frames(self._stats_service)

        if not data_tables or data_tables.robot_human_df.empty or not data_tables.extra:
            return JSONResponse(
                content={"error": "users sent no statistics so far"},
                media_type='application/json',
                status_code=404,
            )

        data = teams_data(data_tables)
        return JSONResponse(
            content=data,
            media_type='application/json'
        )

    async def _dash_teams_post(self, post: DashTeamsGenDashData):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        data_tables = await compose_data_frames(self._stats_service)

        if not data_tables or data_tables.robot_human_df.empty or not data_tables.extra:
            return JSONResponse(
                content={"error": "users sent no statistics so far"},
                media_type='application/json',
                status_code=404,
            )

        data = dashboard_teams(data_tables, post.users_selected)
        return JSONResponse(
            content=data,
            media_type='application/json'
        )

    async def _telemetry_basic(self, data: TelemetryBasicData, request: Request, authorization: str = Header(None)):
        account = self._account_from_bearer(authorization)

        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        ip = request.client.host
        clamp = data.clamp()

        async with ScyllaBatchInserter(self._stats_service) as inserter:
            for record in clamp['records']:
                if clamp['teletype'] == 'network':
                    await inserter.insert(
                        dict(
                            tenant_name=account,
                            ip=ip,
                            enduser_client_version=clamp['enduser_client_version'],
                            counter=record['counter'],
                            error_message=record['error_message'],
                            scope=record['scope'],
                            success=record['success'],
                            url=record['url'],
                            teletype=clamp['teletype'],
                            ts_start=clamp['ts_start'],
                            ts_end=clamp['ts_end']
                        ),
                        to="net"
                    )
                elif clamp['teletype'] == 'robot_human':
                    await inserter.insert(
                        dict(
                            tenant_name=account,
                            ip=ip,
                            enduser_client_version=clamp['enduser_client_version'],

                            completions_cnt=record['completions_cnt'],
                            file_extension=record['file_extension'],
                            human_characters=record['human_characters'],
                            model=record['model'],
                            robot_characters=record['robot_characters'],

                            teletype=clamp['teletype'],
                            ts_end=clamp['ts_end'],
                            ts_start=clamp['ts_start'],
                        ),
                        to="rh"
                    )
                elif clamp['teletype'] == 'comp_counters':
                    await inserter.insert(
                        dict(
                            tenant_name=account,
                            ip=ip,
                            enduser_client_version=clamp['enduser_client_version'],

                            counters_json_text=json.dumps({
                                k: v for k, v in record.items() if k.startswith('after')
                            }),
                            file_extension=record['file_extension'],
                            model=record['model'],
                            multiline=record['multiline'],

                            teletype=clamp['teletype'],
                            ts_start=clamp['ts_start'],
                            ts_end=clamp['ts_end']
                        ),
                        to="comp"
                    )

        return JSONResponse({"retcode": "OK"})

    async def _telemetry_snippets(self, data: TelemetryBasicData, request: Request, authorization: str = Header(None)):
        account = self._account_from_bearer(authorization)

        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        ip = request.client.host
        clamp = data.clamp()

        async with ScyllaBatchInserter(self._stats_service) as inserter:
            for record in clamp['records']:
                await inserter.insert(
                    dict(
                        tenant_name=account,
                        ip=ip,
                        enduser_client_version=clamp['enduser_client_version'],
                        model=record['model'],
                        corrected_by_user=record['corrected_by_user'],
                        remaining_percentage=record['remaining_percentage'],
                        created_ts=record['created_ts'],
                        accepted_ts=record['created_ts'],
                        finished_ts=record['finished_ts'],
                        grey_text=record['grey_text'],
                        cursor_character=record['inputs']['cursor']['character'],
                        cursor_file=record['inputs']['cursor']['file'],
                        cursor_line=record['inputs']['cursor']['line'],
                        multiline=record['inputs']['multiline'],
                        sources=json.dumps(record['inputs']['sources']),
                        teletype=clamp['teletype']
                    ),
                    to="snip"
                )

        return JSONResponse({"retcode": "OK"})


class TabStatisticsRouter(BaseTabStatisticsRouter):
    def __init__(self, session: RefactSession, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._session = session

    def _account_from_bearer(self, authorization: str) -> str:
        try:
            return self._session.header_authenticate(authorization)
        except BaseException as e:
            raise HTTPException(status_code=401, detail=str(e))
