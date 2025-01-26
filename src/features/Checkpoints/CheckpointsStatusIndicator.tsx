import { Text } from "@radix-ui/themes";
import { FileChangedStatus } from "./types";

export const CheckpointsStatusIndicator = ({
  status,
}: {
  status: FileChangedStatus;
}) => {
  const colors = {
    ADDED: "#22C55E",
    MODIFIED: "#F59E0B",
    DELETED: "#EF4444",
  };

  const shortenedStatus = status.split("")[0];

  return (
    <Text
      size="1"
      style={{
        color: colors[status],
      }}
    >
      {shortenedStatus}
    </Text>
  );
};
