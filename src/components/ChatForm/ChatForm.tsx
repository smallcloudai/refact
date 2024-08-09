import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea, TextAreaProps } from "../TextArea";
import { Form } from "./Form";
import { useOnPressedEnter, useIsOnline } from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { Button } from "@radix-ui/themes";
import { ComboBox, type ComboBoxProps } from "../ComboBox";
import {
  ChatContextFile,
  CodeChatModel,
  SystemPrompts,
} from "../../services/refact";
import { FilesPreview } from "./FilesPreview";
import { ChatControls, ChatControlsProps, Checkbox } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { usePreviewFileRequest } from "./usePreviewFileRequest";
import { useConfig } from "../../app/hooks";
import type { FileInfo, Snippet } from "../../features/Chat";

type useCheckboxStateProps = {
  activeFile: FileInfo;
  snippet: Snippet;
  vecdb: boolean;
  ast: boolean;
  allBoxes: boolean;
  chatId: string;
  canUseTools: boolean;
  host: string;
};

const useControlsState = ({
  activeFile,
  snippet,
  vecdb,
  ast,
  allBoxes,
  chatId,
  canUseTools,
  host,
}: useCheckboxStateProps) => {
  const [interacted, setInteracted] = React.useState(false);

  const fullPathWithCursor = useMemo(() => {
    if (activeFile.cursor === null) {
      return activeFile.path;
    }
    return `${activeFile.path}:${activeFile.cursor}`;
  }, [activeFile.path, activeFile.cursor]);

  const fileNameWithLines = useMemo(() => {
    const hasLines = activeFile.line1 !== null && activeFile.line2 !== null;
    if (!hasLines) return activeFile.name;
    return `${activeFile.name}:${activeFile.line1}-${activeFile.line2}`;
  }, [activeFile.name, activeFile.line1, activeFile.line2]);

  const filePathWithLines = useMemo(() => {
    const hasLines = activeFile.line1 !== null && activeFile.line2 !== null;

    if (!hasLines) return activeFile.path;
    return `${activeFile.path}:${activeFile.line1}-${activeFile.line2}`;
  }, [activeFile.path, activeFile.line1, activeFile.line2]);

  const markdown = useMemo(() => {
    return "```" + snippet.language + "\n" + snippet.code + "\n```\n";
  }, [snippet.language, snippet.code]);

  const codeLineCount = useMemo(() => {
    if (snippet.code.length === 0) return 0;
    return snippet.code.split("\n").length;
  }, [snippet.code]);

  const defaultState = useMemo(() => {
    return {
      use_memory: {
        name: "use_memory",
        checked: true,
        label: "Use memory",
        disabled: false,
        hide: true, // !allBoxes,
        info: {
          text: "Uses notes previously written by assistant, to improve on mistakes. Setting this checkbox is equivalent to @local-notes-to-self command in the text.",
          link: "https://docs.refact.ai/features/ai-chat/",
          linkText: "documentation",
        },
      },
      search_workspace: {
        name: "search_workspace",
        checked: false,
        label: "Search workspace",
        disabled: false,
        hide: !vecdb || !allBoxes || canUseTools,
        info: {
          text: "Searches all files in your workspace using vector database, uses the whole text in the input box as a search query. Setting this checkbox is equivalent to @workspace command in the text.",
          link: "https://docs.refact.ai/features/ai-chat/",
          linkText: "documentation",
        },
      },
      file_upload: {
        name: "file_upload",
        checked: !!snippet.code && !!activeFile.name,
        label: "Attach",
        value: filePathWithLines,
        disabled: !activeFile.name,
        fileName: activeFile.name,
        hide: !allBoxes,
        info: {
          text: "Attaches the current file as context. If the file is large, it prefers the code near the current cursor position. Equivalent to @file name.ext:CURSOR_LINE in the text.",
          link: "https://docs.refact.ai/features/ai-chat/",
          linkText: "documentation",
        },
      },
      // lookup_symbols: {
      //   name: "lookup_symbols",
      //   checked: !!snippet.code && !!activeFile.name,
      //   label: "Lookup symbols at cursor",
      //   value: fullPathWithCursor,
      //   disabled: !activeFile.name,
      //   hide: !ast || !allBoxes,
      //   defaultChecked: !!snippet.code && !!activeFile.name,
      //   info: {
      //     text: "Extracts symbols around the cursor position and searches for them in the AST index. Equivalent to @symbols-at file_name.ext:CURSOR_LINE in the text",
      //     link: "https://docs.refact.ai/features/ai-chat/",
      //     linkText: "documentation",
      //   },
      // },
      selected_lines: {
        name: "selected_lines",
        checked: !!snippet.code,
        label: `Selected ${codeLineCount} lines`,
        value: markdown,
        disabled: !snippet.code,
        hide: host === "web",
        info: {
          text: "Adds the currently selected lines as a snippet for analysis or modification. Equivalent to code in triple backticks ``` in the text.",
        },
      },
    } as const;
  }, [
    vecdb,
    allBoxes,
    canUseTools,
    snippet.code,
    activeFile.name,
    filePathWithLines,
    codeLineCount,
    markdown,
    host,
  ]);

  const [checkboxes, setCheckboxes] =
    React.useState<ChatControlsProps["checkboxes"]>(defaultState);

  const reset = useCallback(() => {
    setInteracted(false);
    setCheckboxes(defaultState), [defaultState];
  }, [setCheckboxes, setInteracted, defaultState]);

  const toggleCheckbox = useCallback(
    (name: string, value: boolean | string) => {
      setInteracted(true);
      setCheckboxes((prev) => {
        const checkbox: Checkbox = { ...prev[name], checked: !!value };
        const maybeAddFile: Record<string, Checkbox> =
          name === "lookup_symbols" && !!value
            ? { file_upload: { ...prev.file_upload, checked: true } }
            : { file_upload: prev.file_upload };
        const nextValue = { ...prev, ...maybeAddFile, [name]: checkbox };
        return nextValue;
      });
    },
    [setCheckboxes, setInteracted],
  );

  useEffect(() => {
    setCheckboxes((prev) => {
      // const lookupDisabled = prev.lookup_symbols.checked
      //   ? false
      //   : !activeFile.name;

      const selectedLineDisabled = prev.selected_lines.checked
        ? false
        : !snippet.code;

      const fileUploadDisabled = prev.file_upload.checked
        ? false
        : !activeFile.name;

      const nextValue = {
        ...prev,
        // use_memory: {
        //   ...prev.use_memory,
        //   hide: false,
        // },
        search_workspace: {
          ...prev.search_workspace,
          hide: !vecdb || !allBoxes || canUseTools,
        },
        // lookup_symbols: {
        //   ...prev.lookup_symbols,
        //   value: fullPathWithCursor,
        //   disabled: lookupDisabled,
        //   hide: !ast || !allBoxes,
        //   checked: interacted
        //     ? prev.lookup_symbols.checked
        //     : !!snippet.code && !!activeFile.name,
        // },
        selected_lines: {
          ...prev.selected_lines,
          label: `Selected ${codeLineCount} lines`,
          value: markdown,
          disabled: selectedLineDisabled,
          checked: interacted
            ? prev.selected_lines.checked && !!snippet.code
            : !!snippet.code,
        },
        file_upload: {
          ...prev.file_upload,
          value: filePathWithLines,
          fileName: activeFile.name,
          disabled: fileUploadDisabled,
          hide: !allBoxes,
          checked: interacted
            ? prev.file_upload.checked
            : !!snippet.code && !!activeFile.name,
        },
      };

      return nextValue;
    });
  }, [
    activeFile.name,
    ast,
    codeLineCount,
    fileNameWithLines,
    filePathWithLines,
    fullPathWithCursor,
    interacted,
    markdown,
    snippet.code,
    vecdb,
    allBoxes,
    canUseTools,
  ]);

  useEffect(() => {
    return () => setCheckboxes(defaultState);
  }, [defaultState, chatId]);

  return {
    checkboxes,
    toggleCheckbox,
    reset,
    setInteracted,
  };
};

export type ChatFormProps = {
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
  clearError: () => void;
  error: string | null;
  caps: {
    error: string | null;
    fetching: boolean;
    default_cap: string;
    available_caps: Record<string, CodeChatModel>;
  };
  model: string;
  onSetChatModel: (model: string) => void;
  isStreaming: boolean;
  onStopStreaming: () => void;
  commands: ComboBoxProps["commands"];
  attachFile: FileInfo;
  // hasContextFile: boolean;
  requestCommandsCompletion: ComboBoxProps["requestCommandsCompletion"];
  requestPreviewFiles: (input: string) => void;
  // setSelectedCommand: (command: string) => void;
  filesInPreview: ChatContextFile[];
  selectedSnippet: Snippet;
  // removePreviewFileByName: (name: string) => void;
  onTextAreaHeightChange: TextAreaProps["onTextAreaHeightChange"];
  showControls: boolean;
  requestCaps: () => void;
  prompts: SystemPrompts;
  onSetSystemPrompt: (prompt: SystemPrompts) => void;
  selectedSystemPrompt: SystemPrompts;
  chatId: string;
  canUseTools: boolean;
  setUseTools: (value: boolean) => void;
  useTools: boolean;
};

export const ChatForm: React.FC<ChatFormProps> = ({
  onSubmit,
  onClose,
  className,
  error,
  clearError,
  caps,
  model,
  onSetChatModel,
  isStreaming,
  onStopStreaming,
  commands,
  attachFile,
  requestCommandsCompletion,
  requestPreviewFiles,
  filesInPreview,
  selectedSnippet,
  // removePreviewFileByName,
  onTextAreaHeightChange,
  showControls,
  // TODO: handle re-requesting caps after error
  requestCaps,
  prompts,
  onSetSystemPrompt,
  selectedSystemPrompt,
  chatId,
  canUseTools,
  setUseTools,
  useTools,
}) => {
  const config = useConfig();
  const [value, setValue] = React.useState("");
  // this should re-render when clicking new chat :/
  const { checkboxes, toggleCheckbox, reset, setInteracted } = useControlsState(
    {
      activeFile: attachFile,
      snippet: selectedSnippet,
      vecdb: config.features?.vecdb ?? false,
      ast: config.features?.ast ?? false,
      allBoxes: showControls,
      chatId,
      canUseTools,
      host: config.host,
    },
  );

  usePreviewFileRequest({
    isCommandExecutable: commands.is_cmd_executable,
    requestPreviewFiles: requestPreviewFiles,
    query: value,
    vecdb: config.features?.vecdb ?? false,
    checkboxes,
  });

  useEffect(() => {
    if (
      Object.keys(caps.available_caps).length === 0 &&
      !caps.default_cap &&
      !caps.fetching
    ) {
      requestCaps();
    }
  }, [
    requestCaps,
    caps.available_caps.length,
    caps.default_cap,
    caps.fetching,
    value,
    caps.available_caps,
  ]);

  useEffect(() => {
    if (!showControls) {
      reset();
    }
  }, [showControls, reset]);

  const isOnline = useIsOnline();

  const handleSubmit = useCallback(() => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming && isOnline) {
      const valueIncludingChecks = addCheckboxValuesToInput(
        trimmedValue,
        checkboxes,
        config.features?.vecdb ?? false,
      );
      onSubmit(valueIncludingChecks);
      setValue(() => "");
    }
  }, [
    value,
    isStreaming,
    isOnline,
    checkboxes,
    config.features?.vecdb,
    onSubmit,
  ]);

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleChange = useCallback(
    (command: string) => {
      setInteracted(true);
      setValue(command);
    },
    [setInteracted],
  );

  if (error) {
    return (
      <ErrorCallout mt="2" onClick={clearError} timeout={null}>
        {error}
      </ErrorCallout>
    );
  }

  return (
    <Card mt="1" style={{ position: "relative", flexShrink: 0 }}>
      {!isOnline && <Callout type="info">Offline</Callout>}

      {isStreaming && (
        <Button
          ml="auto"
          color="red"
          title="stop streaming"
          onClick={onStopStreaming}
        >
          Stop
        </Button>
      )}

      <Form
        disabled={isStreaming || !isOnline}
        className={className}
        onSubmit={() => handleSubmit()}
      >
        <FilesPreview
          files={filesInPreview}
          // onRemovePreviewFile={removePreviewFileByName}
        />

        <ComboBox
          commands={commands}
          requestCommandsCompletion={requestCommandsCompletion}
          value={value}
          onChange={handleChange}
          onSubmit={(event) => {
            handleEnter(event);
          }}
          placeholder={
            commands.completions.length > 0 ? "Type @ for commands" : ""
          }
          render={(props) => (
            <TextArea
              data-testid="chat-form-textarea"
              required={true}
              disabled={isStreaming}
              {...props}
              onTextAreaHeightChange={onTextAreaHeightChange}
              autoFocus={true}
            />
          )}
        />
        <Flex gap="2" className={styles.buttonGroup}>
          {onClose && (
            <BackToSideBarButton
              disabled={isStreaming}
              title="return to sidebar"
              size="1"
              onClick={onClose}
            />
          )}
          <PaperPlaneButton
            disabled={isStreaming || !isOnline}
            title="send"
            size="1"
            type="submit"
          />
        </Flex>
      </Form>

      <ChatControls
        host={config.host}
        checkboxes={checkboxes}
        showControls={showControls}
        onCheckedChange={toggleCheckbox}
        selectProps={{
          value: model || caps.default_cap,
          onChange: onSetChatModel,
          options: Object.keys(caps.available_caps),
        }}
        promptsProps={{
          value: selectedSystemPrompt,
          prompts: prompts,
          onChange: onSetSystemPrompt,
        }}
        useTools={useTools}
        canUseTools={canUseTools}
        setUseTools={setUseTools}
      />
    </Card>
  );
};
