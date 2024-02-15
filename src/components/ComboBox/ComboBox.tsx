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
  setSelectedCommand: (command: string) => void;
  selectedCommand: string;
  removePreviewFileByName: (name: string) => void;
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
  setSelectedCommand,
  selectedCommand,
  removePreviewFileByName,
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
    defaultActiveId: undefined,
  });

  const matches = matchSorter(commandsOrArguments, trigger, {
    baseSort: (a, b) => (a.index < b.index ? -1 : 1),
    threshold: 0,
  });

  const hasMatches = !!trigger && !!matches.length;

  const getValueOrTrigger = () => {
    const state = combobox.getState();
    if (state.activeValue) return state.activeValue;
    if (state.activeId) return combobox.item(state.activeId)?.value ?? trigger;
    return trigger;
  };

  React.useEffect(() => {
    if (!ref.current) return;
    if (startPosition === null) return;
    if (!trigger) return;
    requestCommandsCompletion(value, ref.current.selectionStart);
    executeCommand(value, ref.current.selectionStart);

    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [startPosition, trigger, value]);

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
    const first = combobox.first();
    combobox.setActiveId(first);
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

    if (wasDelete && event.key !== "Backspace") {
      setWasDelete(false);
    } else if (event.key === "Backspace") {
      setWasDelete(true);
    }
  };

  const onKeyUp = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (!ref.current) return;

    const state = combobox.getState();
    if (
      !state.activeValue &&
      event.key === "Enter" &&
      !event.shiftKey &&
      !state.open
    ) {
      event.preventDefault();
      event.stopPropagation();
      onSubmit(event);
      setStartPosition(null);
      setTrigger("");
      combobox.hide();
      return;
    }

    if (event.key === "Enter" && event.shiftKey) {
      setTrigger("");
      combobox.hide();
      return;
    }

    if (wasDelete) {
      const maybeCommand = detectCommand(ref.current);
      if (maybeCommand !== null) {
        const maybeCommandWithArguments = maybeCommand.command.split(" ");
        const [command, args] = maybeCommandWithArguments;

        if (!selectedCommand && args) {
          setSelectedCommand(command + " ");
          removePreviewFileByName(args);
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
    }

    if (event.key === "@" && !state.open && !selectedCommand) {
      setTrigger(event.key);
      const start = ref.current.selectionStart - 1;
      setStartPosition(start);
      combobox.setValue("");
      combobox.show();
    }

    const tabOrEnter = event.key === "Tab" || event.key === "Enter";

    const activeValue = getValueOrTrigger();

    const command = selectedCommand ? activeValue : activeValue + " ";

    if (state.open && tabOrEnter && command) {
      event.preventDefault();
      event.stopPropagation();

      const newInput = replaceValue(
        ref.current,
        trigger,
        command,
        startPosition,
      );

      setTrigger(command);
      onChange(newInput);

      setSelectedCommand(selectedCommand ? "" : command);
      combobox.setValue(command);
    }

    if (event.key === "Space" && state.open && commands.includes(trigger)) {
      const newInput = replaceValue(
        ref.current,
        trigger,
        command,
        startPosition,
      );

      event.preventDefault();
      event.stopPropagation();
      onChange(newInput);
      combobox.setValue(trigger + " ");
      setTrigger(trigger + " ");
      setSelectedCommand(trigger + " ");
    }
  };

  const handleChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
    const maybeTrigger = event.target.value
      .substring(
        startPosition ?? event.target.selectionStart - trigger.length,
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
        setSelectedCommand("");
        setTrigger(command);
        setStartPosition(null);
        combobox.hide();
      } else {
        setSelectedCommand(command);
        setTrigger(command);
      }

      const nextValue = replaceValue(textarea, trigger, command, startPosition);

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
