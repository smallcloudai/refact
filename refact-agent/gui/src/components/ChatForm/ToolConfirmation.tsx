import React, { useCallback, useMemo } from "react";
import {
  PATCH_LIKE_FUNCTIONS,
  useAppDispatch,
  useAppSelector,
  useSendChatRequest,
  // useEventsBusForIDE
} from "../../hooks";
import { Card, Button, Text, Flex } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { Link } from "../Link";
import styles from "./ToolConfirmation.module.css";
import { push } from "../../features/Pages/pagesSlice";
// here
import {
  isAssistantMessage,
  isToolCall,
  ToolConfirmationPauseReason,
} from "../../services/refact";

// here
import {
  selectChatId,
  selectMessages,
  setAutomaticPatch,
} from "../../features/Chat";
import {
  selectThreadMessages,
  selectMessagesFromEndNode,
  selectToolConfirmationRequests,
} from "../../features/ThreadMessages";

type ToolConfirmationProps = {
  pauseReasons: ToolConfirmationPauseReason[];
};

const getConfirmationMessage = (
  commands: string[],
  rules: string[],
  types: string[],
  confirmationCommands: string[],
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
      confirmationCommands.length > 1 ? "Commands need" : "Command needs"
    } confirmation: ${confirmationCommands.join(", ")}.\n\nFollowing ${
      denialCommands.length > 1 ? "commands were" : "command was"
    } denied: ${denialCommands.join(
      ", ",
    )}.\n\nAll due to \`\`\`${ruleText}\`\`\` ${
      rules.length > 1 ? "rules" : "rule"
    }.`;
  }
};

// here
export const ToolConfirmation: React.FC<ToolConfirmationProps> = () =>
  // {pauseReasons}
  {
    const dispatch = useAppDispatch();
    // can we remove pause reasons ?
    const chatId = useAppSelector(selectChatId);

    const toolConfirmationRequests = useAppSelector(
      selectToolConfirmationRequests,
    );

    const commands = toolConfirmationRequests.map((reason) => reason.command);
    const rules = toolConfirmationRequests.map((reason) => reason.rule);
    const types = toolConfirmationRequests.map((_) => "confirmation"); // "confirmation" or "denial"
    const toolCallIds = toolConfirmationRequests.map(
      (reason) => reason.tool_call_id,
    );

    const isPatchConfirmation = commands.some((command) =>
      PATCH_LIKE_FUNCTIONS.includes(command),
    );

    // TBD: integration chats?
    // const integrationPaths = toolConfirmationRequests.map(
    //   (reason) => reason.integr_config_path ?? null,
    // );

    // assuming that at least one path out of all objects is not null so we can show up the link
    // const maybeIntegrationPath = integrationPaths.find((path) => path !== null);

    const allConfirmation = types.every((type) => type === "confirmation");
    const confirmationCommands = commands.filter(
      (_, i) => types[i] === "confirmation",
    );
    const denialCommands = commands.filter((_, i) => types[i] === "denial");

    // const { rejectToolUsage, confirmToolUsage } = useSendChatRequest();

    const rejectToolUsage = useCallback((_ids: string[]) => {
      // TODO: create a tool message with the rejected tool call id and a mesage
    }, []);

    const confirmToolUsage = useCallback(() => {
      // TODO: patch thread to add `ft_confirmation_response`
    }, []);

    // this
    const handleAllowForThisChat = () => {
      dispatch(setAutomaticPatch({ chatId, value: true }));
      confirmToolUsage();
    };

    const handleReject = useCallback(() => {
      rejectToolUsage(toolCallIds);
    }, [rejectToolUsage, toolCallIds]);

    const message = getConfirmationMessage(
      commands,
      rules,
      types,
      confirmationCommands,
      denialCommands,
    );

    if (confirmationCommands.length === 0) return null;

    // TODO: this should use the confirmation requests and not the messages
    if (isPatchConfirmation) {
      // TODO: think of multiple toolcalls support
      return (
        <PatchConfirmation
          handleAllowForThisChat={handleAllowForThisChat}
          rejectToolUsage={handleReject}
          confirmToolUsage={confirmToolUsage}
        />
      );
    }

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
            <Flex
              align="baseline"
              gap="1"
              className={styles.ToolConfirmationHeading}
            >
              <Text as="span">⚠️</Text>
              <Text>Model {allConfirmation ? "wants" : "tried"} to run:</Text>
            </Flex>
            {commands.map((command, i) => (
              <Markdown
                key={toolCallIds[i]}
              >{`${"```bash\n"}${command}${"\n```"}`}</Markdown>
            ))}
            <Text className={styles.ToolConfirmationText}>
              <Markdown color="indigo">{message.concat("\n\n")}</Markdown>
              {/* {maybeIntegrationPath && (
                <Text className={styles.ToolConfirmationText} mt="3">
                  You can modify the ruleset on{" "}
                  <Link
                    onClick={() => {
                      dispatch(
                        push({
                          name: "integrations page",
                          integrationPath: maybeIntegrationPath,
                          wasOpenedThroughChat: true,
                        }),
                      );
                    }}
                    color="indigo"
                  >
                    Configuration Page
                  </Link>
                </Text>
              )} */}
            </Text>
          </Flex>
          <Flex align="end" justify="start" gap="2" direction="row">
            <Button
              color="grass"
              variant="surface"
              size="1"
              onClick={confirmToolUsage}
            >
              {allConfirmation ? "Confirm" : "Continue"}
            </Button>
            {allConfirmation && (
              <Button
                color="red"
                variant="surface"
                size="1"
                onClick={handleReject}
              >
                Stop
              </Button>
            )}
          </Flex>
        </Flex>
      </Card>
    );
  };

type PatchConfirmationProps = {
  handleAllowForThisChat: () => void;
  rejectToolUsage: () => void;
  confirmToolUsage: () => void;
};

const PatchConfirmation: React.FC<PatchConfirmationProps> = ({
  handleAllowForThisChat,
  confirmToolUsage,
  rejectToolUsage,
}) => {
  // TODO: this should use the confirmation requests and not the messages

  const messages = useAppSelector(selectThreadMessages);
  const assistantMessages = messages.filter(isAssistantMessage);
  const lastAssistantMessage = useMemo(() => {
    if (!assistantMessages.length) return null;
    return assistantMessages[assistantMessages.length - 1];
  }, [assistantMessages]);

  const toolCalls = Array.isArray(lastAssistantMessage?.ftm_tool_calls)
    ? lastAssistantMessage.ftm_tool_calls.filter(isToolCall)
    : null;

  if (!toolCalls) return;

  const parsedArgsFromToolCall = JSON.parse(
    toolCalls[0].function.arguments,
  ) as {
    path: string;
    tickets: string;
  };
  const extractedFileNameFromPath =
    parsedArgsFromToolCall.path.split(/[/\\]/)[
      parsedArgsFromToolCall.path.split(/[/\\]/).length - 1
    ];
  const messageForPatch = "Patch " + "`" + extractedFileNameFromPath + "`";

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
          <Flex
            align="baseline"
            gap="1"
            className={styles.ToolConfirmationHeading}
          >
            <Text as="span">⚠️</Text>
            <Text>Model wants to apply changes:</Text>
          </Flex>
          <Text className={styles.ToolConfirmationText}>
            <Markdown color="indigo">{messageForPatch.concat("\n\n")}</Markdown>
          </Text>
        </Flex>
        <Flex align="center" justify="between" gap="2" width="100%">
          <Flex gap="2">
            <Button
              color="grass"
              variant="surface"
              size="1"
              onClick={handleAllowForThisChat}
            >
              Allow for This Chat
            </Button>
            <Button
              color="grass"
              variant="surface"
              size="1"
              onClick={confirmToolUsage}
            >
              Allow Once
            </Button>
          </Flex>
          <Button
            color="red"
            variant="surface"
            size="1"
            onClick={rejectToolUsage}
          >
            Stop
          </Button>
        </Flex>
      </Flex>
    </Card>
  );
};
