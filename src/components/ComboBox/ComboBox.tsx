import React, { useCallback, useMemo } from "react";
import { useComboboxStore, Combobox } from "@ariakit/react";
import { getAnchorRect, replaceRange } from "./utils";
import type { TextAreaProps } from "../TextArea/TextArea";
import { Item } from "./Item";
import { Portal } from "../Portal";
import { Popover } from "./Popover";
import { TruncateLeft } from "../Text";
import type { CommandCompletionResponse } from "../../events";

// TODO: move this or replace it with the  expected response from the server

export type ComboBoxProps = {
  commands: CommandCompletionResponse;
  // maybeMove request commands to onchange ?
  onChange: (value: string) => void;
  value: string;
  onSubmit: React.KeyboardEventHandler<HTMLTextAreaElement>;
  placeholder?: string;
  render: (props: TextAreaProps) => React.ReactElement;
  requestCommandsCompletion: (query: string, cursor: number) => void;
};

export const ComboBox: React.FC<ComboBoxProps> = ({
  commands,
  onSubmit,
  placeholder,
  onChange,
  value,
  render,
  requestCommandsCompletion,
}) => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [moveCursorTo, setMoveCursorTo] = React.useState<number | null>(null);

  const combobox = useComboboxStore({
    defaultOpen: false,
    placement: "top-start",
    defaultActiveId: undefined,
  });

  const matches = useMemo(() => commands.completions, [commands.completions]);

  const hasMatches = useMemo(() => {
    return matches.length > 0;
  }, [matches]);

  React.useEffect(() => {
    if (moveCursorTo === null) return;
    if (ref.current) {
      ref.current.setSelectionRange(moveCursorTo, moveCursorTo);
    }
    setMoveCursorTo(null);
    return () => setMoveCursorTo(null);
  }, [moveCursorTo]);

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
    const first = combobox.first();
    combobox.setActiveId(first);
  }, [combobox, hasMatches]);

  React.useEffect(() => {
    combobox.render();
  }, [combobox, value]);

  React.useEffect(() => {
    if (!ref.current) return;
    const cursor = Math.min(
      ref.current.selectionStart,
      ref.current.selectionEnd,
    );
    requestCommandsCompletion(value, cursor);
  }, [requestCommandsCompletion, value]);

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      const state = combobox.getState();

      if (state.open && event.key === "Tab") {
        event.preventDefault();
      }
    },
    [combobox],
  );

  const onKeyUp = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (!ref.current) return;

      const state = combobox.getState();

      if (event.key === "Enter" && !event.shiftKey && !hasMatches) {
        event.stopPropagation();
        onSubmit(event);
        setMoveCursorTo(null);
        return;
      }

      if (event.key === "Enter" && event.shiftKey) {
        return;
      }

      const wasArrowLeftOrRight =
        event.key === "ArrowLeft" || event.key === "ArrowRight";
      if (wasArrowLeftOrRight) {
        combobox.hide();
      }

      if (wasArrowLeftOrRight && state.open) {
        combobox.hide();
      }

      const tabOrEnterOrSpace =
        event.key === "Tab" || event.key === "Enter" || event.key === "Space";

      const command = state.activeValue;

      if (state.open && tabOrEnterOrSpace && command) {
        event.preventDefault();
        event.stopPropagation();
        const nextValue = replaceRange(
          ref.current.value,
          commands.replace,
          command,
        );
        onChange(nextValue);
        setMoveCursorTo(commands.replace[0] + command.length);
      }

      if (event.key === "Escape") {
        combobox.hide();
      }
    },
    [combobox, commands.replace, hasMatches, onChange, onSubmit],
  );

  const handleChange = useCallback(
    (event: React.ChangeEvent<HTMLTextAreaElement>) => {
      onChange(event.target.value);
    },
    [onChange],
  );

  const onItemClick = useCallback(
    (item: string, event: React.MouseEvent<HTMLDivElement>) => {
      event.stopPropagation();
      event.preventDefault();
      const textarea = ref.current;
      if (!textarea) return;
      const nextValue = replaceRange(textarea.value, commands.replace, item);
      onChange(nextValue);
      setMoveCursorTo(commands.replace[0] + item.length);
    },
    [commands.replace, onChange],
  );

  const popoverWidth = ref.current
    ? ref.current.getBoundingClientRect().width - 8
    : null;

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
          onSubmit: onSubmit,
        })}
      />
      <Portal>
        <Popover
          store={combobox}
          hidden={!hasMatches}
          getAnchorRect={() => {
            const textarea = ref.current;
            if (!textarea) return null;
            return getAnchorRect(textarea, ["@", " "]);
          }}
          maxWidth={popoverWidth}
        >
          {matches.map((item, index) => (
            <Item
              key={item + "-" + index}
              value={item}
              onClick={(e) => onItemClick(item, e)}
            >
              <TruncateLeft>{item}</TruncateLeft>
            </Item>
          ))}
        </Popover>
      </Portal>
    </>
  );
};
