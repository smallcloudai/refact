import React from "react";
import type { VecDbStatus } from "../../services/refact";
import { HoverCard, IconButton, Text, DataList } from "@radix-ui/themes";
import classNames from "classnames";
import { LayersIcon } from "@radix-ui/react-icons";
import styles from "./VecDbStatus.module.css";

export const VecDBStatusButton: React.FC<{ status: null | VecDbStatus }> = ({
  status,
}) => {
  if (status === null) {
    return (
      <IconButton disabled loading title="vecdb status">
        <LayersIcon /> Connecting to VecDB
      </IconButton>
    );
  }

  return (
    <HoverCard.Root>
      <HoverCard.Trigger>
        <IconButton
          variant="outline"
          title="Database status"
          className={classNames({
            [styles.vecdb__button__parsing]:
              status.state === "parsing" || status.state === "starting",
          })}
        >
          <LayersIcon />
        </IconButton>
      </HoverCard.Trigger>

      <HoverCard.Content>
        <Text mx="auto">VecDb</Text>
        <DataList.Root size="1">
          <DataList.Item>
            <DataList.Label>Status</DataList.Label>
            <DataList.Value>{status.state}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Unprocessed files</DataList.Label>
            <DataList.Value>{status.files_unprocessed}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Total files</DataList.Label>
            <DataList.Value>{status.files_total}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Database size</DataList.Label>
            <DataList.Value>{status.db_size}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Database cache size</DataList.Label>
            <DataList.Value>{status.db_cache_size}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Request made since start</DataList.Label>
            <DataList.Value>{status.requests_made_since_start}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Vectors made since start</DataList.Label>
            <DataList.Value>{status.vectors_made_since_start}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Queue additions</DataList.Label>
            <DataList.Value>{String(status.queue_additions)}</DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Max files hit</DataList.Label>
            <DataList.Value>
              {String(status.vecdb_max_files_hit)}
            </DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Errors</DataList.Label>
            <DataList.Value>
              {Object.keys(status.vecdb_errors).length}
            </DataList.Value>
          </DataList.Item>
        </DataList.Root>
      </HoverCard.Content>
    </HoverCard.Root>
  );
};
