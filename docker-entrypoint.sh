#!/bin/sh
if [[ ! -v REFACT_DATABASE_HOST ]]; then
    sudo service cassandra start
    REFACT_DATABASE_HOST=127.0.0.1
    REFACT_DATABASE_PORT=9042
fi
python -m self_hosting_machinery.watchdog.docker_watchdog
