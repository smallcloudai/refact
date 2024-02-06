import React from "react";
import { useComboboxStore, Combobox } from "@ariakit/react";
import { matchSorter } from "match-sorter";
import { getAnchorRect, replaceValue } from "./utils";
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
  executeCommand: (command: string) => void;
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
      executeCommand(trigger);
      setTrigger("");
      setSelectedCommand("");
      requestCommandsCompletion("@", 1);
    }
  }, [
    trigger,
    commandIsExecutable,
    executeCommand,
    setSelectedCommand,
    requestCommandsCompletion,
  ]);

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
    }

    if (state.open && event.key === "Tab") {
      event.preventDefault();
    }

    if (event.key === "@" && !state.open && !selectedCommand) {
      setTrigger(event.key);
      requestCommandsCompletion("@", 1);
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

      setTrigger(commandIsExecutable ? "" : command);
      onChange(newInput);

      // if (commandIsExecutable) {
      //   executeCommand(command);
      // }

      setSelectedCommand(selectedCommand ? "" : command);
      requestCommandsCompletion(command, command.length);
      // second box isn't selected by default,
      // combobox.show();
      // maybeExecute(command);
    }

    if (event.key === "Space" && state.open && commands.includes(trigger)) {
      event.preventDefault();
      event.stopPropagation();
      onChange(newInput);
      combobox.setValue(trigger + " ");
      setTrigger(trigger + " ");
      // maybeExecute(trigger + " ");
      // if (commandIsExecutable) {
      //   executeCommand(trigger + " ");
      // }
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
      // console.log("item click");
      const textarea = ref.current;
      if (!textarea) return;
      const command = selectedCommand ? item : item + " ";
      const nextValue = replaceValue(textarea, trigger, command);
      onChange(nextValue);

      // if (commandIsExecutable) {
      //   executeCommand(command);
      // }

      if (selectedCommand) {
        // arguments
        setSelectedCommand("");
        setTrigger(command);
        requestCommandsCompletion(command, command.length);
        // setTrigger("@");

        combobox.hide();
      } else {
        setSelectedCommand(command);
        requestCommandsCompletion(command, command.length);
        setTrigger(command);
        // combobox.hide();
      }
    };

  // console.log(combobox.getState());

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
