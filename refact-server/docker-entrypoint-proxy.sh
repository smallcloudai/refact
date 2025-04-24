#!/bin/sh
if [ -z "$REFACT_DATABASE_HOST" ]; then
    sh database-start.sh
fi
python -m refact_proxy.webgui.webgui
