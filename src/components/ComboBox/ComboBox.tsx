import React from "react";
import {
  useComboboxStore,
  Combobox,
  ComboboxPopover,
  ComboboxItem,
  type ComboboxStore,
} from "@ariakit/react";
import { Box } from "@radix-ui/themes";
import { matchSorter } from "match-sorter";
import {
  getAnchorRect,
  replaceValue,
  // getTriggerOffset,
  type AnchorRect,
} from "./utils";
import { ScrollArea } from "../ScrollArea";
import { Button } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./ComboBox.module.css";
import { TextAreaProps } from "../TextArea/TextArea";

const Item: React.FC<{
  onClick: React.MouseEventHandler<HTMLDivElement>;
  value: string;
  children: React.ReactNode;
}> = ({ children, value, onClick }) => {
  return (
    <Button className={styles.item} variant="ghost" asChild highContrast>
      <ComboboxItem
        value={value}
        onClick={onClick}
        focusOnHover
        clickOnEnter={false}
      >
        {children}
      </ComboboxItem>
    </Button>
  );
};

const Popover: React.FC<
  React.PropsWithChildren & {
    store: ComboboxStore;
    hidden: boolean;
    getAnchorRect: (anchor: HTMLElement | null) => AnchorRect | null;
  }
> = ({ children, ...props }) => {
  return (
    <Box
      asChild
      className={classNames(
        "rt-PopperContent",
        "rt-HoverCardContent",
        styles.popover,
      )}
    >
      <ComboboxPopover unmountOnHide fitViewport {...props}>
        <ScrollArea scrollbars="both" className={styles.popover__scroll}>
          <Box p="1" style={{ overflowY: "hidden", overflowX: "hidden" }}>
            {children}
          </Box>
        </ScrollArea>
      </ComboboxPopover>
    </Box>
  );
};

export type ComboBoxProps = {
  commands: string[];
  commandArguments: string[];
  onChange: (value: string) => void;
  value: string;
  onSubmit: React.KeyboardEventHandler<HTMLTextAreaElement>;
  placeholder?: string;
  render: (props: TextAreaProps) => React.ReactElement;
  requestCommandsCompletion: (
    query: string,
    cursor: number,
    number?: number,
  ) => void;
  executeCommand: (command: string) => void;
  commandIsExecutable: boolean;
};

export const ComboBox: React.FC<ComboBoxProps> = ({
  commands,
  onSubmit,
  placeholder,
  onChange,
  value,
  render,
  commandArguments,
  requestCommandsCompletion,
  executeCommand,
  commandIsExecutable,
}) => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [selectedCommand, setSelectedCommand] = React.useState("");
  const [trigger, setTrigger] = React.useState<string>("");

  const commandsOrArguments = selectedCommand
    ? commandArguments.map((arg) => selectedCommand + arg)
    : commands;

  const combobox = useComboboxStore({
    defaultOpen: false,
    placement: "top-start",
  });

  const matches = matchSorter(commandsOrArguments, trigger, {
    baseSort: (a, b) => (a.index < b.index ? -1 : 1),
  });

  const hasMatches = !!trigger && !!matches.length;

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
  }, [combobox, hasMatches]);

  React.useEffect(() => {
    combobox.render();
  }, [combobox, value]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const state = combobox.getState();

    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      combobox.hide();
    }

    if (state.open && event.key === "Tab") {
      event.preventDefault();
    }

    if (event.key === "@" && !state.open && !selectedCommand) {
      setTrigger(event.key);
      combobox.setValue("");
      combobox.show();
    }
  };

  const onKeyUp = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (!ref.current) return;

    const state = combobox.getState();
    if (!state.activeValue && event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      event.stopPropagation();
      requestCommandsCompletion("@", 1);
      onSubmit(event);
      combobox.hide();
      return;
    }

    const tabOrEnter = event.key === "Tab" || event.key === "Enter";
    const activeValue = state.activeValue ?? "";
    const command = selectedCommand ? activeValue : activeValue + " ";
    const newInput = replaceValue(ref.current, trigger, command);

    if (state.open && tabOrEnter && command) {
      event.preventDefault();
      event.stopPropagation();
      combobox.setValue(command);

      setTrigger(command);
      onChange(newInput);

      if (commandIsExecutable) {
        executeCommand(command);
      }

      setSelectedCommand(selectedCommand ? "" : command);
      requestCommandsCompletion(command, command.length);
    }

    if (event.key === "Space" && state.open && commands.includes(trigger)) {
      event.preventDefault();
      event.stopPropagation();
      onChange(newInput);
      combobox.setValue(trigger + " ");
      setTrigger(trigger + " ");
      if (commandIsExecutable) {
        executeCommand(trigger + " ");
      }
      // combobox.hide();
    }
  };

  const handleChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
    const maybeTrigger = event.target.value
      .substring(
        event.target.selectionStart - (trigger.length + 1),
        event.target.selectionStart,
      )
      .trim();

    onChange(event.target.value);

    if (maybeTrigger && combobox.getState().open) {
      combobox.setValue(maybeTrigger);
      setTrigger(maybeTrigger);
      combobox.show();
    }
  };

  const onItemClick =
    (item: string) => (event: React.MouseEvent<HTMLDivElement>) => {
      event.stopPropagation();
      event.preventDefault();
      const textarea = ref.current;
      if (!textarea) return;
      const command = selectedCommand ? item : item + " ";
      const { selectionStart, selectionEnd } = textarea;
      const start = value.substring(0, selectionStart - trigger.length);
      const end = value.substring(selectionStart, selectionEnd);
      const nextValue = `${start}${command}${end}`;
      onChange(nextValue);

      if (commandIsExecutable) {
        executeCommand(command);
      }

      if (selectedCommand) {
        // arguments
        setSelectedCommand("");
        requestCommandsCompletion("@", 1);
        setTrigger("@");
        combobox.hide();
      } else {
        setSelectedCommand(command);
        requestCommandsCompletion(command, command.length);
        setTrigger(command);
      }
    };

  return (
    <>
      <Combobox
        store={combobox}
        autoSelect
        value={value}
        showOnChange={false}
        showOnKeyDown={false}
        showOnMouseDown={false}
        setValueOnChange={true}
        render={render({
          ref,
          placeholder,
          onScroll: combobox.render,
          onPointerDown: combobox.hide,
          onChange: handleChange,
          onKeyUp: onKeyUp,
          onKeyDown: onKeyDown,
        })}
      />
      <Popover
        store={combobox}
        hidden={!hasMatches}
        getAnchorRect={() => {
          const textarea = ref.current;
          if (!textarea) return null;
          return getAnchorRect(textarea, trigger);
        }}
      >
        {matches.map((item, index) => (
          <Item
            key={item + "-" + index}
            value={item}
            onClick={onItemClick(item)}
          >
            {item.slice(selectedCommand.length)}
          </Item>
        ))}
      </Popover>
    </>
  );
};
