import React from "react";
import type { PauseReason } from "../../features/ToolConfirmation/confirmationSlice";
import { useEventsBusForIDE } from "../../hooks";
import { Card, Button, Text, Flex } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { Link } from "../Link";
import styles from "./ToolConfirmation.module.css";

type ToolConfirmationProps = {
  pauseReasons: PauseReason[];
  onConfirm: () => void;
};

const getConfirmationalMessage = (
  commands: string[],
  rules: string[],
  types: string[],
  confirmationalCommands: string[],
  denialCommands: string[],
) => {
  const ruleText = `${rules.join(", ")} ${rules.length > 1 ? "rules" : "rule"}`;
  if (types.every((type) => type === "confirmation")) {
    return `Following ${
      commands.length > 1 ? "commands need" : "command needs"
    } confirmation due to ${ruleText}.`;
  } else if (types.every((type) => type === "denial")) {
    return `Following ${
      commands.length > 1 ? "commands were" : "command was"
    } denied due to ${ruleText}.`;
  } else {
    return `Following ${
      confirmationalCommands.length > 1 ? "commands need" : "command needs"
    } confirmation: ${confirmationalCommands.join(", ")}.\n\nFollowing ${
      denialCommands.length > 1 ? "commands were" : "command was"
    } denied: ${denialCommands.join(", ")}.\n\nAll due to ${ruleText}.`;
  }
};

export const ToolConfirmation: React.FC<ToolConfirmationProps> = ({
  pauseReasons,
  onConfirm,
}) => {
  const { openIntegrationsFile } = useEventsBusForIDE();

  const commands = pauseReasons.map((reason) => reason.command);
  const rules = pauseReasons.map((reason) => reason.rule);
  const types = pauseReasons.map((reason) => reason.type);
  const toolCallIds = pauseReasons.map((reason) => reason.tool_call_id);

  const allConfirmational = types.every((type) => type === "confirmation");
  const confirmationalCommands = commands.filter(
    (_, i) => types[i] === "confirmation",
  );
  const denialCommands = commands.filter((_, i) => types[i] === "denial");

  const message = getConfirmationalMessage(
    commands,
    rules,
    types,
    confirmationalCommands,
    denialCommands,
  );

  return (
    <Card className={styles.ToolConfirmationCard}>
      <Flex
        align="start"
        justify="between"
        direction="column"
        wrap="wrap"
        gap="4"
      >
        <Flex align="start" direction="column" gap="3" maxWidth="100%">
          <Text className={styles.ToolConfirmationHeading}>
            🔨 Model {allConfirmational ? "wants" : "tried"} to run:
          </Text>
          {commands.map((command, i) => (
            <Markdown
              key={toolCallIds[i]}
            >{`${"```bash\n"}${command}${"\n```"}`}</Markdown>
          ))}
          <Text className={styles.ToolConfirmationText}>
            {message.concat("\n\n")}
            <Text className={styles.ToolConfirmationText}>
              You can modify the ruleset in{" "}
              <Link
                onClick={() => {
                  void openIntegrationsFile();
                }}
              >
                integrations.yaml
              </Link>
            </Text>
          </Text>
        </Flex>
        <Flex align="end" justify="center" gap="1" direction="row">
          <Button color="grass" variant="surface" size="1" onClick={onConfirm}>
            {allConfirmational ? "Confirm" : "Continue"}
          </Button>
        </Flex>
      </Flex>
    </Card>
  );
};
