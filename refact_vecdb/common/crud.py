import traceback

from datetime import datetime

from typing import Dict, Any

from refact_vecdb.common.context import CONTEXT as C


def get_account_data(account: str):
    prep = C.c_session.prepare("SELECT * FROM accounts WHERE account = ?")
    try:
        return C.c_session.execute(prep, [account]).one()
    except Exception:
        traceback.print_exc()
        return {}


def update_account_data(data: Dict[str, Any]) -> None:
    if not get_account_data(data['account']):
        create_account(data['account'])
    prep = C.c_session.prepare("UPDATE accounts SET team = ?, provider = ?, created_ts = ? WHERE account =?")
    C.c_session.execute(
        prep,
        [
            data.get('team'),
            data.get("provider", "gte"),
            data.get("created_ts", datetime.now()),
            data["account"],
        ],
    )


def create_account(account: str):
    prep = C.c_session.prepare("INSERT INTO accounts (account, team, provider, created_ts) VALUES (?,?,?,?)")
    C.c_session.execute(prep, [account, None, 'gte', datetime.now()])

