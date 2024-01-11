#!/bin/sh
if [ -z "$REFACT_DATABASE_HOST" ]; then
    sudo sed -i '/MAX_HEAP_SIZE/c\MAX_HEAP_SIZE="4G"' /etc/cassandra/cassandra-env.sh
    sudo sed -i '/HEAP_NEWSIZE/c\HEAP_NEWSIZE="400M"' /etc/cassandra/cassandra-env.sh

    echo "cassandra database started on localhost"
fi
python -m self_hosting_machinery.watchdog.docker_watchdog
