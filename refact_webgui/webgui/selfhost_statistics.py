import json

from typing import List, Any, Dict

from more_itertools import chunked
from pydantic import BaseModel
from fastapi.responses import StreamingResponse
from fastapi import APIRouter, Request, HTTPException
from fastapi.responses import JSONResponse

from refact_webgui.dashboards.dash_prime import dashboard_prime
from refact_webgui.dashboards.dash_teams import teams_data, dashboard_teams
from refact_webgui.webgui.selfhost_database import StatisticsService
from refact_webgui.webgui.selfhost_database import ScyllaBatchInserter
from refact_webgui.webgui.selfhost_login import RefactSession


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
        self.add_api_route('/rh-stats', self._rh_stats, methods=["GET"])
        self.add_api_route("/telemetry-basic", self._telemetry_basic, methods=["POST"])
        self.add_api_route("/telemetry-snippets", self._telemetry_snippets, methods=["POST"])
        self.add_api_route('/dash-prime', self._dash_prime_get, methods=['GET'])
        self.add_api_route('/dash-teams', self._dash_teams_get, methods=['GET'])
        self.add_api_route('/dash-teams', self._dash_teams_post, methods=['POST'])

    async def _account_dict_from_request(self, request: Request) -> Dict:
        raise NotImplementedError()

    async def _workspace_from_request(self, request: Request) -> str:
        raise NotImplementedError()

    async def _rh_stats(self, request: Request):
        account_dict = await self._account_dict_from_request(request)

        async def streamer():
            records = []
            async for r in self._stats_service.get_robot_human_for_account(**account_dict):
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

    async def _dash_prime_get(self, request: Request):
        data_tables = await self._stats_service.compose_data_frames(
            await self._workspace_from_request(request))

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

    async def _dash_teams_get(self, request: Request):
        data_tables = await self._stats_service.compose_data_frames(
            await self._workspace_from_request(request))

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

    async def _dash_teams_post(self, post: DashTeamsGenDashData, request: Request):
        data_tables = await self._stats_service.compose_data_frames(
            await self._workspace_from_request(request))

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

    async def _telemetry_basic(self, data: TelemetryBasicData, request: Request):
        account_dict = await self._account_dict_from_request(request)

        ip = request.client.host
        clamp = data.clamp()

        async with ScyllaBatchInserter(self._stats_service) as inserter:
            for record in clamp['records']:
                if clamp['teletype'] == 'network':
                    await inserter.insert(
                        dict(
                            ip=ip,
                            enduser_client_version=clamp['enduser_client_version'],
                            counter=record['counter'],
                            error_message=record['error_message'],
                            scope=record['scope'],
                            success=record['success'],
                            url=record['url'],
                            teletype=clamp['teletype'],
                            ts_start=clamp['ts_start'],
                            ts_end=clamp['ts_end'],
                            **account_dict,
                        ),
                        to="net"
                    )
                elif clamp['teletype'] == 'robot_human':
                    await inserter.insert(
                        dict(
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
                            **account_dict,
                        ),
                        to="rh"
                    )
                elif clamp['teletype'] == 'comp_counters':
                    await inserter.insert(
                        dict(
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
                            ts_end=clamp['ts_end'],
                            **account_dict,
                        ),
                        to="comp"
                    )

        return JSONResponse({"retcode": "OK"})

    async def _telemetry_snippets(self, data: TelemetryBasicData, request: Request):
        account_dict = await self._account_dict_from_request(request)

        ip = request.client.host
        clamp = data.clamp()

        async with ScyllaBatchInserter(self._stats_service) as inserter:
            for record in clamp['records']:
                await inserter.insert(
                    dict(
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
                        teletype=clamp['teletype'],
                        **account_dict,
                    ),
                    to="snip"
                )

        return JSONResponse({"retcode": "OK"})


class TabStatisticsRouter(BaseTabStatisticsRouter):
    def __init__(self, session: RefactSession, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._session = session

    async def _account_dict_from_request(self, request: Request) -> Dict:
        try:
            authorization = request.headers.get("Authorization", None)
            return {
                "tenant_name": self._session.header_authenticate(authorization),
            }
        except BaseException as e:
            raise HTTPException(status_code=401, detail=str(e))

    async def _workspace_from_request(self, request: Request) -> str:
        return ""
