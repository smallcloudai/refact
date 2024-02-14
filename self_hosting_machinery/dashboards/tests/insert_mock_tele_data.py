import json
import uuid
import random
import copy

from datetime import datetime, timedelta
from dataclasses import field, dataclass
from typing import Dict, List, Any, Optional, Iterator, Iterable

import faker
import asyncio
from tqdm import tqdm

from self_hosting_machinery.webgui.selfhost_database import RefactDatabase
from self_hosting_machinery.webgui.selfhost_database import StatisticsService
from self_hosting_machinery.webgui.selfhost_database import TelemetryRobotHuman
from self_hosting_machinery.webgui.selfhost_database import TelemetryCompCounters
from self_hosting_machinery.webgui.selfhost_database import TelemetryNetwork


LANGUAGES = [
    {"language": "JavaScript", "tags": ["website", "plugins"], "extension": ".js"},
    {"language": "Python", "tags": ["website", "self-host", "enterprise"], "extension": ".py"},
    {"language": "Java", "tags": ["enterprise"], "extension": ".java"},
    {"language": "C#", "tags": ["enterprise"], "extension": ".cs"},
    {"language": "PHP", "tags": ["website"], "extension": ".php"},
    {"language": "TypeScript", "tags": ["website", "plugins"], "extension": ".ts"},
    {"language": "C++", "tags": ["enterprise"], "extension": ".cpp"},
    {"language": "C", "tags": ["enterprise"], "extension": ".c"},
    {"language": "Ruby", "tags": ["website"], "extension": ".rb"},
    {"language": "Swift", "tags": ["website", "enterprise"], "extension": ".swift"},
    {"language": "Kotlin", "tags": ["enterprise"], "extension": ".kt"},
    {"language": "Go", "tags": ["enterprise"], "extension": ".go"},
    {"language": "Rust", "tags": ["enterprise"], "extension": ".rs"},
    {"language": "Shell", "tags": ["self-host"], "extension": ".sh"},
    {"language": "Objective-C", "tags": ["website", "enterprise"], "extension": ".m"},
    {"language": "Dart", "tags": ["website", "plugins"], "extension": ".dart"},
    {"language": "Scala", "tags": ["enterprise"], "extension": ".scala"},
    {"language": "Perl", "tags": ["self-host"], "extension": ".pl"},
    {"language": "Haskell", "tags": ["self-host"], "extension": ".hs"},
    {"language": "Lua", "tags": ["self-host"], "extension": ".lua"}
]


@dataclass
class MockUser:
    email: str = field(default_factory=lambda: faker.Faker().email())
    ip: str = field(default_factory=lambda: faker.Faker().ipv4())
    name: str = field(default_factory=lambda: faker.Faker().name())
    bearer: str = field(default_factory=lambda: str(uuid.uuid4())[:12])
    workday_hours: int = field(default_factory=lambda: random.randint(6, 12))
    procrastination_chance: float = field(default_factory=lambda: random.random() * 0.15)
    team: Optional[str] = None
    languages: Optional[List[Dict[str, Any]]] = None
    plugin: str = field(default_factory=lambda: random.choice(["vscode", "jetbrains"]))
    works_on_weekends: bool = field(default_factory=lambda: random.random() < 0.3)


@dataclass
class MockModel:
    model: str
    robot_score_multiplier: float
    model_betterness: float


def mock_robot_human(dt: datetime, user: MockUser, model: MockModel) -> Dict[str, Any]:
    records = [
        {
            "completions_cnt": (completions_cnt := random.randint(3, 20)),
            "file_extension": l["extension"],
            "human_characters": completions_cnt * 30,
            "model": model.model,
            "robot_characters": int(completions_cnt * 30 * random.randint(3, 10) / 10) * model.robot_score_multiplier,
        }
        for l in user.languages
    ]
    big_json = {
        "tenant_name": user.email,
        "team": user.team,
        "ip": user.ip,
        "enduser_client_version": user.plugin,
        "records": records,
        "teletype": "robot_human",
        "ts_start": int(dt.timestamp()),
        "ts_end": int((dt + timedelta(minutes=random.randint(1, 10))).timestamp()),
    }
    return big_json


def mock_comp_counters(robot_human_json: Dict[str, Any], user: MockUser, model: MockModel) -> Dict[str, Any]:
    remainings_base = {
        "100": .15,
        "80_100": .20,
        "50_80": .30,
        "0_50": .20,
        "0": .15,
    }

    def adjust_remanings(rem: Dict, mult: float):
        remainings = copy.deepcopy(rem)
        remainings = {
            "100": remainings["100"] * mult,
            "80_100": remainings["80_100"] * mult,
            "50_80": remainings["50_80"],
            "0_50": remainings["0_50"],
            "0": remainings["0"],
        }

        # Normalize the percentages to ensure the sum is 1
        total_remainings = sum(remainings.values())
        remainings = {key: value / total_remainings for key, value in remainings.items()}
        return remainings

    mutline_percentage = random.random() * 0.5
    records = []
    for rh_rec in robot_human_json["records"]:
        for c in range(rh_rec["completions_cnt"]):
            is_multiline = random.random() < mutline_percentage
            file_extension = rh_rec["file_extension"]
            new_rem_base = adjust_remanings(remainings_base, model.model_betterness)
            rec = {}
            for after_val, after_mult in {
                30: 1,
                90:  random.randint(6, 10) / 10,
                180: random.randint(6, 9) / 10,
                360: random.randint(6, 8) / 10,
            }.items():
                rem_base_adj = adjust_remanings(new_rem_base, after_mult)
                rec_rand = random.random()
                broken = False
                for rem_val, rem_mult in rem_base_adj.items():
                    rec.setdefault(f"after_{after_val}s_remaining_{rem_val}", 0)
                    if rec_rand < rem_mult and not broken:
                        rec[f"after_{after_val}s_remaining_{rem_val}"] = 1
                        broken = True
                        continue

            rec["file_extension"] = file_extension
            rec["model"] = rh_rec["model"]
            rec["multiline"] = is_multiline
            records.append(rec)

    # group records using key: (file_extesion, model, multiline)
    grouped_records = {}
    for rec in records:
        key = (rec["file_extension"], rec["model"], rec["multiline"])
        grouped_records.setdefault(key, [])
        grouped_records[key].append(rec)

    # sum the records
    records = []
    for key, recs in grouped_records.items():
        rec = {}
        for k in recs[0].keys():
            if k in ["file_extension", "model", "multiline"]:
                rec[k] = recs[0][k]
            else:
                # import IPython; IPython.embed(); quit()
                rec[k] = sum([r[k] for r in recs])
        records.append(rec)

    big_json = {
        "tenant_name": user.email,
        "team": user.team,
        "ip": user.ip,
        "enduser_client_version": user.plugin,
        "records": records,
        "teletype": "comp_counters",
        "ts_start": robot_human_json["ts_start"],
        "ts_end": robot_human_json["ts_end"],
    }
    return big_json


def mock_network(robot_human_json: Dict[str, Any], user: MockUser) -> Dict[str, Any]:
    scope_types = [
        {"scope": "completion", "url_suffix": "/v1/completions", "proba": 1},
        {"scope": "chat", "url_suffix": "/v1/chat", "proba": 0.5},
        {"scope": "toolbox", "url_suffix": "/v1/toolbox", "proba": 0.5},
        {"scope": "login", "url_suffix": "/v1/login", "proba": 1},
    ]
    records = []
    proba_error = 0.03
    for scope_type in scope_types:
        record = {}
        if random.random() > scope_type["proba"]:
            continue

        record["url"] = f"http://localhost/{scope_type['url_suffix']}"
        record["error_message"] = ""
        record["scope"] = scope_type["scope"]
        record["success"] = True
        record["counter"] = random.randint(1, 10)

        if random.random() < proba_error:
            record["success"] = False
            record["error_message"] = f"error occured with scope={scope_type['scope']}; ERROR_CODE=E{random.randint(1, 20)}"
            records.append(record)
            if record["scope"] == "login":
                break
            continue

        if scope_type["scope"] == "completion":
            record["counter"] = sum([r["completions_cnt"] for r in robot_human_json["records"]]) * 15

        records.append(record)
    big_json = {
        "tenant_name": user.email,
        "team": user.team,
        "ip": user.ip,
        "enduser_client_version": user.plugin,
        "records": records,
        "teletype": "network",
        "ts_start": robot_human_json["ts_start"],
        "ts_end": robot_human_json["ts_end"],
    }
    return big_json


def insert_robot_human(stats_service: StatisticsService, records: Iterable[Dict[str, Any]]):
    for record in records:
        for r_rec in record["records"]:
            stats_service.robot_human_insert(
                TelemetryRobotHuman(
                    tenant_name=record["tenant_name"],
                    team=record["team"],
                    ip=record["ip"],
                    enduser_client_version=record["enduser_client_version"],
                    completions_cnt=r_rec["completions_cnt"],
                    file_extension=r_rec["file_extension"],
                    human_characters=r_rec["human_characters"],
                    model=r_rec["model"],
                    robot_characters=r_rec["robot_characters"],
                    teletype=record["teletype"],
                    ts_start=record["ts_start"],
                    ts_end=record["ts_end"],
                )
            )


def insert_comp_counters(stats_service: StatisticsService, records: Iterable[Dict[str, Any]]):
    for record in records:
        for r_rec in record["records"]:
            stats_service.comp_counters_insert(
                TelemetryCompCounters(
                    tenant_name=record["tenant_name"],
                    team=record["team"],
                    ip=record["ip"],
                    enduser_client_version=record["enduser_client_version"],
                    counters_json_text=json.dumps({
                        k: v for k, v in r_rec.items() if k.startswith('after')
                    }),
                    file_extension=r_rec["file_extension"],
                    model=r_rec["model"],
                    multiline=r_rec["multiline"],
                    teletype=record["teletype"],
                    ts_start=record["ts_start"],
                    ts_end=record["ts_end"],
                )
            )


def insert_network(stats_service: StatisticsService, records: Iterable[Dict[str, Any]]):
    for record in records:
        for r_rec in record["records"]:
            stats_service.network_insert(
                TelemetryNetwork(
                    tenant_name=record["tenant_name"],
                    team=record["team"],
                    ip=record["ip"],
                    enduser_client_version=record["enduser_client_version"],
                    counter=r_rec["counter"],
                    error_message=r_rec["error_message"],
                    scope=r_rec["scope"],
                    success=r_rec["success"],
                    url=r_rec["url"],
                    teletype=record["teletype"],
                    ts_start=record["ts_start"],
                    ts_end=record["ts_end"],
                )
            )


def this_day_users_send_telemetry(dt: datetime, users: List[MockUser], model: MockModel) -> Iterator[Dict[str, Any]]:
    workday_begins_at = 9
    workday_ends_at = 22

    workday_hours = workday_ends_at - workday_begins_at

    for user in users:
        user_chance_to_send_telemetry = 1 * (8 / user.workday_hours)
        hours_worked = 0

        if not user.works_on_weekends and dt.weekday() in [5, 6]:
            continue
        if random.random() < user.procrastination_chance:
            continue
        for hour in range(workday_hours):
            if random.random() > user_chance_to_send_telemetry or hours_worked >= user.workday_hours:
                continue
            hours_worked += 1

            rh = mock_robot_human(dt, user, model)
            yield rh
            yield mock_comp_counters(rh, user, model)
            yield mock_network(rh, user)


def this_day(stats_service: StatisticsService, dt: datetime, users: List[MockUser], model: MockModel):
    for record in this_day_users_send_telemetry(dt, users, model):
        if record["teletype"] == "robot_human":
            insert_robot_human(stats_service, [record])
        elif record["teletype"] == "comp_counters":
            insert_comp_counters(stats_service, [record])
        elif record["teletype"] == "network":
            insert_network(stats_service, [record])


def main():
    database = RefactDatabase()
    loop = asyncio.get_event_loop()
    loop.run_until_complete(database.connect())

    stats_service = StatisticsService(database)
    stats_service.init_models()

    teams = ["website", "plugins", "self-host", "enterprise"]
    mock_users = [MockUser() for _ in range(12)]
    for user in mock_users:
        user.team = random.choice(teams)
        user.languages = random.sample(
           (population := [l for l in LANGUAGES if user.team in l["tags"]]), random.randint(1, min(4, len(population)))
        )

    dt_start = datetime.now() - timedelta(days=62)
    dt_end = datetime.now() - timedelta(days=2)

    models = [
        {"model": MockModel("refact-0", 1, 1.), "release_date": dt_start + timedelta(days=-14)},
        {"model": MockModel("refact-1", 1.3, 2.), "release_date": dt_start + timedelta(days=14)},
        {"model": MockModel("refact-2", 1.5, 3.), "release_date": dt_start + timedelta(days=28)},
        {"model": MockModel("refact-3", 1.8, 4.), "release_date": dt_start + timedelta(days=42)},
        {"model": MockModel("refact-4", 2, 5.), "release_date": dt_start + timedelta(days=56)},
    ]

    # iterate over all days, creat dt = day at 00:00:00 + 9 hours
    for current_date in tqdm(range((dt_end - dt_start).days + 1), desc="Processing Days"):
        dt = datetime(dt_start.year, dt_start.month, dt_start.day, 0, 0, 0)
        dt += timedelta(days=current_date, hours=9)

        model = [m["model"] for m in models if m["release_date"] <= dt][-1]

        this_day(stats_service, dt, mock_users, model)


if __name__ == '__main__':
    main()
