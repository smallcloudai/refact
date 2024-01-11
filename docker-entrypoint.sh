#!/bin/sh
REFACT_CASSANDRA_DIR="$REFACT_PERM_DIR/cassandra"
if [ ! -d "$REFACT_CASSANDRA_DIR" ]; then
    mkdir -p "$REFACT_CASSANDRA_DIR"
    chown cassandra:cassandra "$REFACT_CASSANDRA_DIR"
    if [ ! -z "$(ls /var/lib/cassandra)" ]; then
        cp -rp /var/lib/cassandra/* "$REFACT_CASSANDRA_DIR"
    fi
    cp -rp /var/log/cassandra "$REFACT_CASSANDRA_DIR/log"
fi
if [ -z "$REFACT_DATABASE_HOST" ]; then
    # patch cassandra config to work with REFACT_CASSANDRA_DIR
    sed -i "s|/var/lib/cassandra|$REFACT_CASSANDRA_DIR|g" /etc/cassandra/cassandra.yaml
    # patch cassandra.in.sh for less memory consumption and logging to REFACT_CASSANDRA_DIR/log
    REFACT_CASSANDRA_INCLUDE=/usr/sbin/cassandra.in.sh
    cp /usr/share/cassandra/cassandra.in.sh "$REFACT_CASSANDRA_INCLUDE"
    echo "MAX_HEAP_SIZE=4G" >> "$REFACT_CASSANDRA_INCLUDE"
    echo "HEAP_NEWSIZE=400M" >> "$REFACT_CASSANDRA_INCLUDE"
    echo "CASSANDRA_LOG_DIR=$REFACT_CASSANDRA_DIR/log" >> "$REFACT_CASSANDRA_INCLUDE"

    if [ ! -z "$(service cassandra status | grep 'not running')" ]; then
        service cassandra start
        echo "cassandra database started on localhost"
    fi
fi
python -m self_hosting_machinery.watchdog.docker_watchdog
