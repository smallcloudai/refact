import React from "react";
import { Flex, IconButton } from "@radix-ui/themes";
import { ReloadIcon } from "@radix-ui/react-icons";
import { LinkButton } from "../../components/Buttons";
import styles from "./AgentUsage.module.css";

interface AgentUsageActionsProps {
  plan: string;
  refetchUser: () => Promise<void>;
  startPollingForUser: () => void;
}

export const AgentUsageActions: React.FC<AgentUsageActionsProps> = ({
  plan,
  refetchUser,
  startPollingForUser,
}) => {
  const isPlanFree = plan === "FREE";
  const buttonHref = isPlanFree
    ? "https://refact.smallcloud.ai/pro"
    : "https://refact.smallcloud.ai/link-tbd";

  const buttonText = isPlanFree ? "Upgrade to PRO" : "Increase limit";

  return (
    <Flex gap="2" justify="end">
      <IconButton
        size="2"
        variant="outline"
        title="Refetch limits data"
        onClick={() => void refetchUser()}
      >
        <ReloadIcon />
      </IconButton>
      <LinkButton
        size="2"
        variant="outline"
        href={buttonHref}
        target="_blank"
        onClick={startPollingForUser}
        className={styles.upgrade_button}
      >
        {buttonText}
      </LinkButton>
    </Flex>
  );
};
