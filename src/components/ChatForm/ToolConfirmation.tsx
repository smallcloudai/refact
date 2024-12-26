import React from "react";
import type { PauseReason } from "../../features/ToolConfirmation/confirmationSlice";
import {
  useAppDispatch,
  useSendChatRequest,
  // useEventsBusForIDE
} from "../../hooks";
import { Card, Button, Text, Flex } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { Link } from "../Link";
import styles from "./ToolConfirmation.module.css";
import { push } from "../../features/Pages/pagesSlice";

type ToolConfirmationProps = {
  pauseReasons: PauseReason[];
};

const getConfirmationalMessage = (
  commands: string[],
  rules: string[],
  types: string[],
  confirmationalCommands: string[],
  denialCommands: string[],
) => {
  const ruleText = `${rules.join(", ")}`;
  if (types.every((type) => type === "confirmation")) {
    return `${
      commands.length > 1 ? "Commands need" : "Command needs"
    } confirmation due to \`\`\`${ruleText}\`\`\` ${
      rules.length > 1 ? "rules" : "rule"
    }.`;
  } else if (types.every((type) => type === "denial")) {
    return `${
      commands.length > 1 ? "Commands were" : "Command was"
    } denied due to \`\`\`${ruleText}\`\`\` ${
      rules.length > 1 ? "rules" : "rule"
    }.`;
  } else {
    return `${
      confirmationalCommands.length > 1 ? "Commands need" : "Command needs"
    } confirmation: ${confirmationalCommands.join(", ")}.\n\nFollowing ${
      denialCommands.length > 1 ? "commands were" : "command was"
    } denied: ${denialCommands.join(
      ", ",
    )}.\n\nAll due to \`\`\`${ruleText}\`\`\` ${
      rules.length > 1 ? "rules" : "rule"
    }.`;
  }
};

export const ToolConfirmation: React.FC<ToolConfirmationProps> = ({
  pauseReasons,
}) => {
  const dispatch = useAppDispatch();

  // const { openIntegrationsFile } = useEventsBusForIDE();

  const commands = pauseReasons.map((reason) => reason.command);
  const rules = pauseReasons.map((reason) => reason.rule);
  const types = pauseReasons.map((reason) => reason.type);
  const toolCallIds = pauseReasons.map((reason) => reason.tool_call_id);

  const allConfirmational = types.every((type) => type === "confirmation");
  const confirmationalCommands = commands.filter(
    (_, i) => types[i] === "confirmation",
  );
  const denialCommands = commands.filter((_, i) => types[i] === "denial");

  const { rejectToolUsage, confirmToolUsage } = useSendChatRequest();

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
            <Markdown>{message.concat("\n\n")}</Markdown>
            <Text className={styles.ToolConfirmationText} mt="3">
              You can modify the ruleset on{" "}
              <Link
                onClick={() => {
                  dispatch(push({ name: "integrations page" }));
                }}
              >
                Configuration Page
              </Link>
            </Text>
          </Text>
        </Flex>
        <Flex align="end" justify="start" gap="2" direction="row">
          <Button
            color="grass"
            variant="surface"
            size="1"
            onClick={confirmToolUsage}
          >
            {allConfirmational ? "Confirm" : "Continue"}
          </Button>
          {allConfirmational && (
            <Button
              color="red"
              variant="surface"
              size="1"
              onClick={rejectToolUsage}
            >
              Deny
            </Button>
          )}
        </Flex>
      </Flex>
    </Card>
  );
};
