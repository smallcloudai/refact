#!/bin/sh
if [[ ! -v REFACT_DATABASE_HOST ]]; then
    sudo service cassandra start
fi
python -m self_hosting_machinery.watchdog.docker_watchdog
