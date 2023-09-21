import traceback

from datetime import datetime

from typing import Dict, Any, List

from refact_vecdb.common.context import CONTEXT as C


def get_account_data(account: str) -> Dict[str, Any]:
    session = C.c_session
    try:
        return session.execute(
            session.prepare("SELECT * FROM accounts WHERE account = ?"),
            [account]
        ).one() or {}
    except Exception:
        traceback.print_exc()
        return {}


def update_account_data(data: Dict[str, Any]) -> None:
    session = C.c_session

    if not get_account_data(data['account']):
        create_account(data['account'])
    session.execute(
        session.prepare("UPDATE accounts SET team = ?, provider = ?, created_ts = ? WHERE account =?"),
        [
            data.get('team'),
            data.get("provider", "gte"),
            data.get("created_ts", datetime.now()),
            data["account"],
        ],
    )


def create_account(account: str) -> None:
    session = C.c_session

    prep = session.prepare("INSERT INTO accounts (account, team, provider, created_ts) VALUES (?,?,?,?)")
    session.execute(prep, [account, None, 'gte', datetime.now()])


def get_all_providers() -> List[str]:
    session = C.c_session

    providers = set()
    for row in session.execute("SELECT provider FROM accounts"):
        providers.add(row['provider'])
    return list(providers)
