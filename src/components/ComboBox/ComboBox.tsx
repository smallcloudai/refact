import React from "react";
import { useComboboxStore, Combobox } from "@ariakit/react";
import { matchSorter } from "match-sorter";
import { getAnchorRect, replaceValue, detectCommand } from "./utils";
import type { TextAreaProps } from "../TextArea/TextArea";
import { Item } from "./Item";
import { Portal } from "../Portal";
import { Popover } from "./Popover";
import { useUndoRedo } from "../../hooks";

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
    trigger: string | null,
    number?: number,
  ) => void;
  setSelectedCommand: (command: string) => void;
  selectedCommand: string;
  removePreviewFileByName: (name: string) => void;
};

export const ComboBox: React.FC<ComboBoxProps> = ({
  commands,
  onSubmit,
  placeholder,
  onChange: _onChange,
  value: _value,
  render,
  commandArguments,
  requestCommandsCompletion,
  setSelectedCommand,
  selectedCommand,
  removePreviewFileByName,
}) => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [trigger, setTrigger] = React.useState<string>("");
  const [startPosition, setStartPosition] = React.useState<null | number>(null);
  const [wasDelete, setWasDelete] = React.useState<boolean>(false);
  const [endPosition, setEndPosition] = React.useState<null | number>(null);
  const undoRedo = useUndoRedo(_value);
  const value = undoRedo.state;

  const onChange = (value: string) => {
    undoRedo.setState(value);
    _onChange(value);
  };

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
    const maybeTrigger = !selectedCommand && trigger ? trigger : null;
    requestCommandsCompletion(value, ref.current.selectionStart, maybeTrigger);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [startPosition, trigger, value]);

  React.useLayoutEffect(() => {
    if (!ref.current) return;
    if (endPosition === null) return;
    ref.current.setSelectionRange(endPosition, endPosition);
    setEndPosition(null);
  }, [endPosition]);

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

    if (!ref.current) return;

    const isMod = event.metaKey || event.ctrlKey;
    if (
      isMod &&
      event.key === "z" &&
      !event.shiftKey &&
      undoRedo.isUndoPossible
    ) {
      event.preventDefault();
      undoRedo.undo();
      const maybeCommand = detectCommand(ref.current);
      if (maybeCommand?.command) {
        setTrigger(maybeCommand.command);
        setWasDelete(true);
      }
    }

    if (
      isMod &&
      event.key === "z" &&
      event.shiftKey &&
      undoRedo.isRedoPossible
    ) {
      event.preventDefault();
      const nextValue = undoRedo.futureStates[0];
      const clonedTextArea = {
        ...ref.current,
        value: nextValue,
      };
      const maybeCommand = detectCommand(clonedTextArea);
      undoRedo.redo();
      if (maybeCommand?.command) {
        setTrigger(maybeCommand.command);
      }
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
      } else {
        setTrigger("");
        setSelectedCommand("");
        setStartPosition(null);
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
      onChange(newInput.value);
      setEndPosition(newInput.endPosition);
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
      onChange(newInput.value);
      setEndPosition(newInput.endPosition);
      combobox.setValue(trigger + " ");
      setTrigger(trigger + " ");
      setSelectedCommand(trigger + " ");
    }
  };

  const handleChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
    onChange(event.target.value);
    const maybeCommand = detectCommand(event.target);
    if (maybeCommand) {
      setTrigger(maybeCommand.command);
      setStartPosition(maybeCommand.startPosition);
      const [command, ...args] = maybeCommand.command
        .split(" ")
        .filter((d) => d);
      setSelectedCommand(args.length > 0 ? command + " " : "");
    } else {
      setTrigger("");
      setSelectedCommand("");
      setStartPosition(null);
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

      onChange(nextValue.value);
      setEndPosition(nextValue.endPosition);
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
      <Portal>
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
      </Portal>
    </>
  );
};
