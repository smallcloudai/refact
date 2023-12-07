import os
import time
import uuid
import logging

from typing import List, Dict, Any, Optional
from datetime import datetime

from sqlalchemy import Column, Integer, Boolean, DateTime, Text
from sqlalchemy.orm import DeclarativeBase
from sqlalchemy.orm import sessionmaker
from sqlalchemy.orm import Session
from sqlalchemy import create_engine
from sqlalchemy_utils import database_exists, create_database


class Base(DeclarativeBase):
    pass


class TelemetryNetwork(Base):
    __tablename__ = 'telemetry_network'

    id = Column(Text, primary_key=True)
    tenant_name = Column(Text)
    team = Column(Text, default="")
    ts_reported = Column(DateTime)
    ip = Column(Text)
    enduser_client_version = Column(Text)

    counter = Column(Integer)
    error_message = Column(Text)
    scope = Column(Text)
    success = Column(Boolean)
    url = Column(Text)

    teletype = Column(Text)
    ts_start = Column(Integer)
    ts_end = Column(Integer)


class TelemetrySnippets(Base):
    __tablename__ = 'telemetry_snippets'

    id = Column(Text, primary_key=True)
    tenant_name = Column(Text)
    team = Column(Text, default="")
    ts_reported = Column(DateTime)
    ip = Column(Text)
    enduser_client_version = Column(Text)

    model = Column(Text)
    corrected_by_user = Column(Text)
    remaining_percentage = Column(Text)
    created_ts = Column(Integer)
    accepted_ts = Column(Integer)
    finished_ts = Column(Integer)
    grey_text = Column(Text)
    cursor_character = Column(Integer)
    cursor_file = Column(Text)
    cursor_line = Column(Integer)
    multiline = Column(Boolean)
    sources = Column(Text)

    teletype = Column(Text)


class TelemetryRobotHuman(Base):
    __tablename__ = 'telemetry_robot_human'

    id = Column(Text, primary_key=True)
    tenant_name = Column(Text)
    team = Column(Text, default="")
    ts_reported = Column(DateTime)
    ip = Column(Text)
    enduser_client_version = Column(Text)

    completions_cnt = Column(Integer)
    file_extension = Column(Text)
    human_characters = Column(Integer)
    model = Column(Text)
    robot_characters = Column(Integer)

    teletype = Column(Text)
    ts_start = Column(Integer)
    ts_end = Column(Integer)


class TelemetryCompCounters(Base):
    __tablename__ = 'telemetry_comp_counters'

    id = Column(Text, primary_key=True)
    tenant_name = Column(Text)
    team = Column(Text, default="")
    ts_reported = Column(DateTime)
    ip = Column(Text)
    enduser_client_version = Column(Text)

    counters_json_text = Column(Text)
    file_extension = Column(Text)
    model = Column(Text)
    multiline = Column(Boolean)

    teletype = Column(Text)
    ts_end = Column(Integer)
    ts_start = Column(Integer)


class DisableLogger:

    def __enter__(self):
        logging.disable(logging.CRITICAL)

    def __exit__(self, exit_type, exit_value, exit_traceback):
        logging.disable(logging.NOTSET)


class RefactDatabase:

    def __init__(self):
        # NOTE: this is a hack to wait for a db to be ready
        self._session: Optional[Session] = None
        while True:
            try:
                with DisableLogger():
                    url = os.environ.get("REFACT_DATABASE_URL", "postgresql://postgres:postrges@localhost:5432")
                    engine = create_engine(f"{url}/refact")
                    if not database_exists(engine.url):
                        create_database(engine.url)
                    Base.metadata.create_all(engine)
                    self._session = sessionmaker(bind=engine)()
                    break
            except Exception as e:
                logging.warning(f"Database problem {e}, sleep for 10 seconds...")
                time.sleep(10)

    def __del__(self):
        if self._session:
            self._session.close()

    @property
    def session(self) -> Optional[Session]:
        return self._session


class StatisticsService:

    def __init__(self, database: RefactDatabase):
        self._database = database

    def network_insert(self, telemetry_snippets: TelemetrySnippets):
        telemetry_snippets.id = str(uuid.uuid1())
        telemetry_snippets.ts_reported = datetime.now()
        self._database.session.add(telemetry_snippets)
        self._database.session.commit()

    def robot_human_insert(self, telemetry_robot_human: TelemetryRobotHuman):
        telemetry_robot_human.id = str(uuid.uuid1())
        telemetry_robot_human.ts_reported = datetime.now()
        self._database.session.add(telemetry_robot_human)
        self._database.session.commit()

    def comp_counters_insert(self, telemetry_comp_counters: TelemetryCompCounters):
        telemetry_comp_counters.id = str(uuid.uuid1())
        telemetry_comp_counters.ts_reported = datetime.now()
        self._database.session.add(telemetry_comp_counters)
        self._database.session.commit()

    def network_select_all(self) -> List[Dict[str, Any]]:
        return [
            {
                field: getattr(row, field)
                for field in TelemetryNetwork.__table__.columns.keys()
            } for row in self._database.session.query(TelemetryNetwork).all()
        ]

    def snippets_select_all(self) -> List[Dict[str, Any]]:
        return [
            {
                field: getattr(row, field)
                for field in TelemetrySnippets.__table__.columns.keys()
            } for row in self._database.session.query(TelemetrySnippets).all()
        ]

    def robot_human_select_all(self) -> List[Dict[str, Any]]:
        return [
            {
                field: getattr(row, field)
                for field in TelemetryRobotHuman.__table__.columns.keys()
            } for row in self._database.session.query(TelemetryRobotHuman).all()
        ]

    def comp_counters_select_all(self) -> List[Dict[str, Any]]:
        return [
            {
                field: getattr(row, field)
                for field in TelemetryCompCounters.__table__.columns.keys()
            } for row in self._database.session.query(TelemetryCompCounters).all()
        ]
