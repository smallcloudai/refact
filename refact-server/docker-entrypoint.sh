#!/bin/sh
if [ -z "$REFACT_DATABASE_HOST" ]; then
    sh database-start.sh
fi
python -m self_hosting_machinery.watchdog.docker_watchdog
