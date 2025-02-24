from typing import List
from prompt_toolkit.application import Application
from prompt_toolkit.layout import Layout, HSplit
from prompt_toolkit.layout.containers import Window

apps: List[Application] = []
# we are keeping a list of the layouts so we can clear the layout between
# transitions, and get them back later.
layouts: List[Layout] = []


async def start_app(app: Application):
    apps.append(app)
    layouts.append(app.layout)
    while len(apps) > 0:
        await apps[-1].run_async()


def empty_layout():
    return Layout(HSplit([Window()]))


def push_app(app: Application):
    current_app = apps[-1]
    apps.append(app)
    layouts.append(current_app.layout)

    layouts[-2] = current_app.layout
    current_app.layout = empty_layout()
    current_app.invalidate()
    current_app.exit()


def pop_app():
    current_app = apps.pop()
    layouts[-1] = current_app.layout
    current_app.layout = empty_layout()
    current_app.invalidate()
    current_app.exit()
    current_app.layout = layouts.pop()
    apps[-1].layout = layouts[-1]


def exit_all_apps():
    if len(apps) > 0:
        current_app = apps.pop()
        current_app.layout = empty_layout()
        current_app.invalidate()
        current_app.exit()
    apps.clear()
    layouts.clear()
