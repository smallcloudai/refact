import React, { CSSProperties, useCallback, useMemo } from "react";
import {
  Text,
  Flex,
  HoverCard,
  Link,
  Skeleton,
  Box,
  Switch,
  Badge,
} from "@radix-ui/themes";
import { Select } from "../Select";
import { type Config } from "../../features/Config/configSlice";
import { TruncateLeft } from "../Text";
import styles from "./ChatForm.module.css";
import classNames from "classnames";
import { PromptSelect } from "./PromptSelect";
import { Checkbox } from "../Checkbox";
import {
  ExclamationTriangleIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { useTourRefs } from "../../features/Tour";
import { ToolUseSwitch } from "./ToolUseSwitch";
import {
  ToolUse,
  selectAutomaticPatch,
  selectChatId,
  selectCheckpointsEnabled,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectThreadMode,
  selectToolUse,
  setAutomaticPatch,
  setChatMode,
  setEnabledCheckpoints,
  setToolUse,
} from "../../features/Chat/Thread";
import { useAppSelector, useAppDispatch, useCapsForToolUse } from "../../hooks";
import { getChatById } from "../../features/History/historySlice";

export const ApplyPatchSwitch: React.FC = () => {
  const dispatch = useAppDispatch();
  const isPatchAutomatic = useAppSelector(selectAutomaticPatch);

  const handleAutomaticPatchChange = (checked: boolean) => {
    dispatch(setAutomaticPatch(checked));
  };

  return (
    <Flex
      gap="4"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
      justify="between"
    >
      <Text size="2" mr="auto">
        Patch files without confirmation
      </Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable automatic patch calls by Agent"
          checked={isPatchAutomatic}
          onCheckedChange={handleAutomaticPatchChange}
        />
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content size="2" maxWidth="280px">
            <Text weight="bold">Enabled</Text>
            <Text as="p" size="2">
              When enabled, Refact Agent will automatically apply changes to
              files without asking for your confirmation.
            </Text>
            <Text as="div" mt="2" weight="bold">
              Disabled
            </Text>
            <Text as="p" size="2">
              When disabled, Refact Agent will ask for your confirmation before
              applying any unsaved changes.
            </Text>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
};
export const AgentRollbackSwitch: React.FC = () => {
  const dispatch = useAppDispatch();
  const isAgentRollbackEnabled = useAppSelector(selectCheckpointsEnabled);

  const handleAgentRollbackChange = (checked: boolean) => {
    dispatch(setEnabledCheckpoints(checked));
  };

  return (
    <Flex
      gap="4"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
      justify="between"
    >
      <Text size="2" mr="auto">
        Changes rollback
      </Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable automatic patch calls by Agent"
          checked={isAgentRollbackEnabled}
          onCheckedChange={handleAgentRollbackChange}
        />
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content size="2" maxWidth="280px">
            <Flex direction="column" gap="2">
              <Text as="p" size="2">
                When enabled, Refact Agent will automatically make snapshots of
                files between your messages
              </Text>
              <Text as="p" size="2">
                You can rollback file changes to checkpoints taken when you sent
                messages to Agent
              </Text>
              <Badge
                color="yellow"
                asChild
                style={{
                  whiteSpace: "pre-wrap",
                }}
              >
                <Flex gap="2" p="2" align="center">
                  <ExclamationTriangleIcon
                    width={16}
                    height={16}
                    style={{ flexGrow: 1, flexShrink: 0 }}
                  />
                  <Text as="p" size="1">
                    Warning: may slow down performance of Agent in large
                    projects
                  </Text>
                </Flex>
              </Badge>
            </Flex>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
};

export const ReasoningModeSwitch: React.FC = () => {
  const DEFAULT_MODE = "AGENT";
  const ALLOWED_MODES_TO_DISPLAY_SWITCH = ["AGENT", "THINKING_AGENT"];
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const currentMode = useAppSelector(selectThreadMode);
  const modeFromHistory = useAppSelector(
    (state) => getChatById(state, chatId)?.mode ?? DEFAULT_MODE,
    { devModeChecks: { stabilityCheck: "never" } },
  );

  const isReasoningEnabled = currentMode === "THINKING_AGENT";

  const handleReasoningModeChange = (checked: boolean) => {
    // TODO: if default mode wasn't agent, but configure or project summary, what should we do?
    const newMode = checked
      ? "THINKING_AGENT"
      : modeFromHistory === "THINKING_AGENT"
        ? DEFAULT_MODE
        : modeFromHistory;

    dispatch(setChatMode(newMode));
  };

  const tooltipStyles: CSSProperties = {
    marginLeft: 4,
    visibility: "hidden",
    opacity: 0,
  };

  if (currentMode && !ALLOWED_MODES_TO_DISPLAY_SWITCH.includes(currentMode)) {
    return null;
  }

  return (
    <Flex
      gap="2"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      justify="between"
      width="100%"
    >
      <Text size="2">Use a o3-mini reasoning model for planning</Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable reasoner for Agent"
          checked={isReasoningEnabled}
          onCheckedChange={handleReasoningModeChange}
        />
        <QuestionMarkCircledIcon style={tooltipStyles} />
      </Flex>
    </Flex>
  );
};

const CapsSelect: React.FC = () => {
  const refs = useTourRefs();
  const caps = useCapsForToolUse();

  const allDisabled = caps.usableModelsForPlan.every((option) => {
    if (typeof option === "string") return false;
    return option.disabled;
  });

  return (
    <Flex
      gap="2"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
      ref={(x) => refs.setUseModel(x)}
    >
      <Text size="2">Use model:</Text>
      <Skeleton loading={caps.loading}>
        <Box>
          {allDisabled ? (
            <Text size="1" color="gray">
              No models available
            </Text>
          ) : (
            <Select
              title="chat model"
              options={caps.usableModelsForPlan}
              value={caps.currentModel}
              onChange={caps.setCapModel}
            />
          )}
        </Box>
      </Skeleton>
    </Flex>
  );
};

type CheckboxHelp = {
  text: string;
  link?: string;
  linkText?: string;
};

export type Checkbox = {
  name: string;
  label: string;
  checked: boolean;
  value?: string;
  disabled: boolean;
  fileName?: string;
  hide?: boolean;
  info?: CheckboxHelp;
};

export type ChatControlsProps = {
  checkboxes: Record<string, Checkbox>;
  onCheckedChange: (
    name: keyof ChatControlsProps["checkboxes"],
    checked: boolean | string,
  ) => void;

  host: Config["host"];
};

const ChatControlCheckBox: React.FC<{
  name: string;
  checked: boolean;
  disabled?: boolean;
  onCheckChange: (value: boolean | string) => void;
  label: string;
  fileName?: string;
  infoText?: string;
  href?: string;
  linkText?: string;
}> = ({
  name,
  checked,
  disabled,
  onCheckChange,
  label,
  fileName,
  infoText,
  href,
  linkText,
}) => {
  return (
    <Flex justify="between">
      <Checkbox
        size="1"
        name={name}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onCheckChange}
      >
        {label}
        {fileName && (
          // TODO: negative margin ?
          <Flex ml="-3px">
            <TruncateLeft>{fileName}</TruncateLeft>
          </Flex>
        )}
      </Checkbox>
      {infoText && (
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content maxWidth="240px" size="1">
            <Flex direction="column" gap="4">
              <Text as="div" size="1">
                {infoText}
              </Text>

              {href && linkText && (
                <Text size="1">
                  Read more on our{" "}
                  <Link size="1" href={href}>
                    {linkText}
                  </Link>
                </Text>
              )}
            </Flex>
          </HoverCard.Content>
        </HoverCard.Root>
      )}
    </Flex>
  );
};

export const ChatControls: React.FC<ChatControlsProps> = ({
  checkboxes,
  onCheckedChange,
  host,
}) => {
  const refs = useTourRefs();
  const dispatch = useAppDispatch();
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const messages = useAppSelector(selectMessages);
  const toolUse = useAppSelector(selectToolUse);
  const onSetToolUse = useCallback(
    (value: ToolUse) => dispatch(setToolUse(value)),
    [dispatch],
  );

  const showControls = useMemo(
    () => messages.length === 0 && !isStreaming && !isWaiting,
    [isStreaming, isWaiting, messages],
  );

  return (
    <Flex
      pt="2"
      pb="2"
      gap="2"
      direction="column"
      align="start"
      className={classNames(styles.controls)}
    >
      {Object.entries(checkboxes).map(([key, checkbox]) => {
        if (host === "web" && checkbox.name === "file_upload") {
          return null;
        }
        if (checkbox.hide === true) {
          return null;
        }
        return (
          <ChatControlCheckBox
            key={key}
            name={checkbox.name}
            label={checkbox.label}
            checked={checkbox.checked}
            disabled={checkbox.disabled}
            onCheckChange={(value) => onCheckedChange(key, value)}
            infoText={checkbox.info?.text}
            href={checkbox.info?.link}
            linkText={checkbox.info?.linkText}
            fileName={checkbox.fileName}
          />
        );
      })}

      {showControls && (
        <Flex gap="2" direction="column">
          <ToolUseSwitch
            ref={(x) => refs.setUseTools(x)}
            toolUse={toolUse}
            setToolUse={onSetToolUse}
          />
          <CapsSelect />
          <PromptSelect />
        </Flex>
      )}
    </Flex>
  );
};
