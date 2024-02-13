import React from "react";
import { useComboboxStore, Combobox } from "@ariakit/react";
import { matchSorter } from "match-sorter";
import { getAnchorRect, replaceValue, detectCommand } from "./utils";
import type { TextAreaProps } from "../TextArea/TextArea";
import { Item } from "./Item";
import { Popover } from "./Popover";

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
  executeCommand: (command: string, cursor: number) => void;
  commandIsExecutable: boolean;
  setSelectedCommand: (command: string) => void;
  selectedCommand: string;
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
  setSelectedCommand,
  selectedCommand,
}) => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [trigger, setTrigger] = React.useState<string>("");
  const [startPosition, setStartPosition] = React.useState<null | number>(null);
  const [wasDelete, setWasDelete] = React.useState<boolean>(false);

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

  React.useEffect(() => {
    if (trigger && commandIsExecutable) {
      const place = (startPosition ?? 0) + trigger.length - 1;
      executeCommand(value, place);
    }
  }, [trigger, commandIsExecutable, executeCommand, startPosition, value]);

  React.useEffect(() => {
    if (trigger) {
      requestCommandsCompletion(trigger, trigger.length);
    } else {
      requestCommandsCompletion("@", 1);
    }
  }, [trigger, requestCommandsCompletion]);

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
  }, [combobox, hasMatches]);

  React.useEffect(() => {
    combobox.render();
  }, [combobox, value]);

  React.useEffect(() => {
    if (!trigger && selectedCommand) {
      setSelectedCommand("");
    }
  }, [trigger, setSelectedCommand, selectedCommand]);

  // TODO: if selected value changes and box is open set activeId to first item

  const onKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const state = combobox.getState();

    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      combobox.hide();
      setStartPosition(null);
    }

    if (state.open && event.key === "Tab") {
      event.preventDefault();
    }

    if (event.key === "@" && !state.open && !selectedCommand) {
      setTrigger(event.key);
      const start = ref.current ? ref.current.selectionStart : null;
      setStartPosition(start);
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
      setStartPosition(null);
      combobox.hide();
      return;
    }

    if (event.key === "Enter" && event.shiftKey) {
      setTrigger("");
      combobox.hide();
      return;
    }

    const tabOrEnter = event.key === "Tab" || event.key === "Enter";
    const activeValue = state.activeValue ?? trigger;

    const command = selectedCommand ? activeValue : activeValue + " ";

    if (state.open && tabOrEnter && command) {
      event.preventDefault();
      event.stopPropagation();

      const newInput = replaceValue(
        startPosition,
        ref.current,
        trigger,
        command,
      );

      setTrigger(command);
      onChange(newInput);

      setSelectedCommand(selectedCommand ? "" : command);
      combobox.setValue(command);
    }

    if (event.key === "Space" && state.open && commands.includes(trigger)) {
      const newInput = replaceValue(
        startPosition,
        ref.current,
        trigger,
        command,
      );

      event.preventDefault();
      event.stopPropagation();
      onChange(newInput);
      combobox.setValue(trigger + " ");
      setTrigger(trigger + " ");
      setSelectedCommand(trigger + " ");
    }

    if (event.key === "Backspace") {
      setWasDelete(true);
      const maybeCommand = detectCommand(ref.current);

      if (maybeCommand !== null) {
        const maybeCommandWithArguments = maybeCommand.command.split(" ");
        const [command, args] = maybeCommandWithArguments;

        if (!selectedCommand && args) {
          setSelectedCommand(command + " ");
        } else if (selectedCommand && maybeCommandWithArguments.length < 2) {
          setSelectedCommand("");
        }

        setTrigger(maybeCommand.command);
        setStartPosition(maybeCommand.startPosition);
        combobox.show();
      } else {
        setTrigger("");
        setSelectedCommand("");
      }
    } else if (wasDelete) {
      setWasDelete(false);
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

    if (trigger && maybeTrigger) {
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

      if (selectedCommand) {
        // arguments
        setSelectedCommand("");
        setTrigger(command);
        combobox.hide();
      } else {
        setSelectedCommand(command);
        setTrigger(command);
      }

      const nextValue = replaceValue(startPosition, textarea, trigger, command);
      onChange(nextValue);
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
