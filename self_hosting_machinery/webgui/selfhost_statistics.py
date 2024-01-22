import json
import asyncio

from typing import List, Any, Dict

import aiohttp
from pydantic import BaseModel
from fastapi import APIRouter, Request
from fastapi.responses import JSONResponse

from self_hosting_machinery.webgui.selfhost_database import StatisticsService
from self_hosting_machinery.webgui.selfhost_database import TelemetryNetwork
from self_hosting_machinery.webgui.selfhost_database import TelemetrySnippets
from self_hosting_machinery.webgui.selfhost_database import TelemetryRobotHuman
from self_hosting_machinery.webgui.selfhost_database import TelemetryCompCounters


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

    def clamp(self) -> Dict[str, Any]:
        return {
            "users_selected": self.users_selected,
        }


class TabStatisticsRouter(APIRouter):

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
        self.add_api_route("/telemetry-basic", self._telemetry_basic, methods=["POST"])
        self.add_api_route("/telemetry-snippets", self._telemetry_snippets, methods=["POST"])
        self.add_api_route('/dash-prime', self._dash_prime_get, methods=['GET'])
        self.add_api_route('/dash-teams', self._dash_teams_get, methods=['GET'])
        self.add_api_route('/dash-teams', self._dash_teams_post, methods=['POST'])

    async def _dash_prime_get(self):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get('http://localhost:8010/dash-prime/plots-data') as resp:
                    resp_json = await resp.json()
            return JSONResponse(content=resp_json, media_type='application/json', status_code=resp.status)
        except Exception as e:
            return JSONResponse(
                content={
                    'error': str(e)
                },
                media_type='application/json',
                status_code=500)

    async def _dash_teams_get(self):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get('http://localhost:8010/dash-teams/plots-data') as resp:
                    resp_json = await resp.json()
            return JSONResponse(content=resp_json, media_type='application/json', status_code=resp.status)
        except Exception as e:
            return JSONResponse(
                content={
                    'error': str(e)
                },
                media_type='application/json',
                status_code=500)

    async def _dash_teams_post(self, data: DashTeamsGenDashData):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post('http://localhost:8010/dash-teams/plots-data', json=data.clamp()) as resp:
                    resp_json = await resp.json()
            return JSONResponse(content=resp_json, media_type='application/json', status_code=resp.status)
        except Exception as e:
            return JSONResponse(
                content={
                    'error': str(e)
                },
                media_type='application/json',
                status_code=500)

    async def _telemetry_basic(self, data: TelemetryBasicData, request: Request, account: str = "user"):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        ip = request.client.host
        clamp = data.clamp()

        for record in clamp['records']:
            await asyncio.sleep(0)
            if clamp['teletype'] == 'network':
                self._stats_service.network_insert(
                    TelemetryNetwork(
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
                    )
                )
            elif clamp['teletype'] == 'robot_human':
                self._stats_service.robot_human_insert(
                    TelemetryRobotHuman(
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
                    )
                )
            elif clamp['teletype'] == 'comp_counters':
                self._stats_service.comp_counters_insert(
                    TelemetryCompCounters(
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
                    )
                )

        return JSONResponse({"retcode": "OK"})

    async def _telemetry_snippets(self, data: TelemetryBasicData, request: Request, account: str = "user"):
        if not self._stats_service.is_ready:
            return self._stats_service_not_available_response

        ip = request.client.host
        clamp = data.clamp()
        if not clamp['records']:
            return JSONResponse({"retcode": "OK"})

        for record in clamp['records']:
            self._stats_service.snippets_insert(
                TelemetrySnippets(
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
                )
            )

        return JSONResponse({"retcode": "OK"})
