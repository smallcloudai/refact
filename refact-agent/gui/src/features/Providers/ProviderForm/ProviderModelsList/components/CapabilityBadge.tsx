import { Badge } from "@radix-ui/themes";
import { CheckIcon, Cross1Icon } from "@radix-ui/react-icons";
import { FC } from "react";

type CapabilityBadgeProps = {
  name: string;
  enabled: boolean;
  onClick?: () => void;
  interactive?: boolean;
};

/**
 * Reusable component for model capability badges
 */
export const CapabilityBadge: FC<CapabilityBadgeProps> = ({
  name,
  enabled,
  onClick,
  interactive = true,
}) => {
  return (
    <Badge
      color={enabled ? "green" : "gray"}
      onClick={interactive ? onClick : undefined}
      style={interactive ? { cursor: "pointer" } : undefined}
    >
      {name}{" "}
      {enabled ? <CheckIcon width="12px" /> : <Cross1Icon width="12px" />}
    </Badge>
  );
};
