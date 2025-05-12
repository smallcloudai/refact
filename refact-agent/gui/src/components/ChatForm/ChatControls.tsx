import React, { useCallback, useMemo } from "react";
import {
  Text,
  Flex,
  HoverCard,
  Link,
  Skeleton,
  Box,
  Switch,
  Badge,
  Button,
  DataList,
} from "@radix-ui/themes";
import { Select, type SelectProps } from "../Select";
import { type Config } from "../../features/Config/configSlice";
import { TruncateLeft } from "../Text";
import styles from "./ChatForm.module.css";
import classNames from "classnames";
import { PromptSelect } from "./PromptSelect";
import { Checkbox } from "../Checkbox";
import {
  ExclamationTriangleIcon,
  LockClosedIcon,
  LockOpen1Icon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { useTourRefs } from "../../features/Tour";
import { ToolUseSwitch } from "./ToolUseSwitch";
import {
  ToolUse,
  selectAreFollowUpsEnabled,
  selectAutomaticPatch,
  selectChatId,
  selectCheckpointsEnabled,
  selectIsStreaming,
  selectIsTitleGenerationEnabled,
  selectIsWaiting,
  selectMessages,
  selectToolUse,
  setAreFollowUpsEnabled,
  setIsTitleGenerationEnabled,
  setAutomaticPatch,
  setEnabledCheckpoints,
  setToolUse,
} from "../../features/Chat/Thread";
import { useAppSelector, useAppDispatch, useCapsForToolUse } from "../../hooks";
import { useAttachedFiles } from "./useCheckBoxes";
import { toPascalCase } from "../../utils/toPascalCase";
import { Coin } from "../../images";
import { push } from "../../features/Pages/pagesSlice";

export const ApplyPatchSwitch: React.FC = () => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const isPatchAutomatic = useAppSelector(selectAutomaticPatch);

  const handleAutomaticPatchChange = (checked: boolean) => {
    dispatch(setAutomaticPatch({ chatId, value: checked }));
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
          title="Enable/disable changed rollback made by Agent"
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
export const FollowUpsSwitch: React.FC = () => {
  const dispatch = useAppDispatch();
  const areFollowUpsEnabled = useAppSelector(selectAreFollowUpsEnabled);

  const handleFollowUpsEnabledChange = (checked: boolean) => {
    dispatch(setAreFollowUpsEnabled(checked));
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
        Follow-Ups messages
      </Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable follow-ups messages generation by Agent"
          checked={areFollowUpsEnabled}
          onCheckedChange={handleFollowUpsEnabledChange}
        />
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content size="2" maxWidth="280px">
            <Flex direction="column" gap="2">
              <Text as="p" size="2">
                When enabled, Refact Agent will automatically generate related
                follow-ups to the conversation
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
                    Warning: may increase coins spending
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

export const TitleGenerationSwitch: React.FC = () => {
  const dispatch = useAppDispatch();
  const isTitleGenerationEnabled = useAppSelector(
    selectIsTitleGenerationEnabled,
  );

  const handleTitleGenerationEnabledChange = (checked: boolean) => {
    dispatch(setIsTitleGenerationEnabled(checked));
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
        Chat Titles
      </Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable chat titles generation by Agent"
          checked={isTitleGenerationEnabled}
          onCheckedChange={handleTitleGenerationEnabledChange}
        />
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content size="2" maxWidth="280px">
            <Flex direction="column" gap="2">
              <Text as="p" size="2">
                When enabled, Refact Agent will automatically generate
                summarized chat title for the conversation
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
                    Warning: may increase coins spending
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

export const CapsSelect: React.FC<{ disabled?: boolean }> = ({ disabled }) => {
  const refs = useTourRefs();
  const caps = useCapsForToolUse();
  const dispatch = useAppDispatch();

  const handleAddNewModelClick = useCallback(() => {
    dispatch(push({ name: "providers page" }));
  }, [dispatch]);

  const onSelectChange = useCallback(
    (value: string) => {
      if (value === "add-new-model") {
        handleAddNewModelClick();
        return;
      }
      caps.setCapModel(value);
    },
    [handleAddNewModelClick, caps],
  );

  const optionsWithToolTips: SelectProps["options"] = useMemo(() => {
    // Map existing models with tooltips
    const modelOptions = caps.usableModelsForPlan.map((option) => {
      if (!caps.data) return option;
      if (!caps.data.metadata) return option;
      if (!caps.data.metadata.pricing) return option;
      if (!option.value.startsWith("refact/")) return option;
      const key = option.value.replace("refact/", "");
      if (!(key in caps.data.metadata.pricing)) return option;
      const pricingForModel = caps.data.metadata.pricing[key];
      const tooltip = (
        <Flex direction="column" gap="4">
          <Text size="1">Cost per Million Tokens</Text>
          <DataList.Root size="1" trim="both" className={styles.data_list}>
            {Object.entries(pricingForModel).map(([key, value]) => {
              return (
                <DataList.Item key={key} align="stretch">
                  <DataList.Label minWidth="88px">
                    {toPascalCase(key)}
                  </DataList.Label>
                  <DataList.Value className={styles.data_list__value}>
                    <Flex justify="between" align="center" gap="2">
                      {value * 1_000} <Coin width="12px" height="12px" />
                    </Flex>
                  </DataList.Value>
                </DataList.Item>
              );
            })}
          </DataList.Root>
        </Flex>
      );
      return {
        ...option,
        tooltip,
        // title,
      };
    });

    return [
      ...modelOptions,
      { type: "separator" },
      {
        value: "add-new-model",
        textValue: "Add new model",
      },
    ];
  }, [caps.data, caps.usableModelsForPlan]);

  const allDisabled = caps.usableModelsForPlan.every((option) => {
    if (typeof option === "string") return false;
    return option.disabled;
  });

  return (
    <Flex
      gap="2"
      align="center"
      wrap="wrap"
      // flexGrow="1"
      // flexShrink="0"
      // width="100%"
      ref={(x) => refs.setUseModel(x)}
    >
      <Skeleton loading={caps.loading}>
        <Box>
          {allDisabled ? (
            <Text size="1" color="gray">
              No models available
            </Text>
          ) : (
            <Select
              title="chat model"
              options={optionsWithToolTips}
              value={caps.currentModel}
              onChange={onSelectChange}
              disabled={disabled}
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
  locked?: boolean;
};

export type ChatControlsProps = {
  checkboxes: Record<string, Checkbox>;
  onCheckedChange: (
    name: keyof ChatControlsProps["checkboxes"],
    checked: boolean | string,
  ) => void;

  host: Config["host"];
  attachedFiles: ReturnType<typeof useAttachedFiles>;
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
  locked?: boolean;
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
  locked,
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
        {locked && <LockClosedIcon opacity="0.6" />}
        {locked === false && <LockOpen1Icon opacity="0.6" />}
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
  attachedFiles,
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
            locked={checkbox.locked}
          />
        );
      })}

      {host !== "web" && (
        <Button
          title="Attach current file"
          onClick={attachedFiles.addFile}
          disabled={!attachedFiles.activeFile.name || attachedFiles.attached}
          size="1"
          radius="medium"
        >
          Attach: {attachedFiles.activeFile.name}
        </Button>
      )}

      {showControls && (
        <Flex gap="2" direction="column">
          <ToolUseSwitch
            ref={(x) => refs.setUseTools(x)}
            toolUse={toolUse}
            setToolUse={onSetToolUse}
          />
          {/* <CapsSelect /> */}
          <PromptSelect />
        </Flex>
      )}
    </Flex>
  );
};
