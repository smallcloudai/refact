#!/bin/sh
if [ -z "$REFACT_DATABASE_HOST" ]; then
    sudo MAX_HEAP_SIZE=4G HEAP_NEWSIZE=200M service cassandra start
    echo "cassandra database started on localhost"
fi
python -m self_hosting_machinery.watchdog.docker_watchdog
