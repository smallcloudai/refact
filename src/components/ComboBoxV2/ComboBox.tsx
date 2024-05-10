import React, { useCallback, useMemo } from "react";
import { useComboboxStore, Combobox } from "@ariakit/react";
import { getAnchorRect } from "./utils";
import type { TextAreaProps } from "../TextArea/TextArea";
import { Item } from "./Item";
import { Portal } from "../Portal";
import { Popover } from "./Popover";
import { TruncateLeft } from "../Text";

// TODO: move this or replace it with the  expected response from the server
export type Commands = {
  completions: string[];
  replace: [number, number];
  is_cmd_executable: boolean;
};

function replaceRange(
  str: string,
  range: [number, number],
  replacement: string,
) {
  const sortedRange = [
    Math.min(range[0], range[1]),
    Math.max(range[0], range[1]),
  ];
  return str.slice(0, sortedRange[0]) + replacement + str.slice(sortedRange[1]);
}

function modifyValueAndDispatchChange(
  textarea: HTMLTextAreaElement,
  commands: Commands,
  command: string,
) {
  const nextValue = replaceRange(textarea.value, commands.replace, command);
  Object.getOwnPropertyDescriptor(
    window.HTMLTextAreaElement.prototype,
    "value",
  )?.set?.call(textarea, nextValue);
  textarea.dispatchEvent(
    new Event("change", {
      bubbles: true,
    }),
  );
}

export type ComboBoxProps = {
  commands: Commands;
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

  const combobox = useComboboxStore({
    defaultOpen: false,
    placement: "top-start",
    defaultActiveId: undefined,
  });

  const matches = useMemo(() => commands.completions, [commands.completions]);

  const hasMatches = useMemo(() => {
    return matches.length > 0;
  }, [matches]);

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
    // const first = combobox.first();
    // combobox.setActiveId(first);
  }, [combobox, hasMatches]);

  React.useEffect(() => {
    combobox.render();
  }, [combobox, value]);

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
        // combobox.hide();
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
        modifyValueAndDispatchChange(ref.current, commands, command);
      }

      if (event.key === "Escape") {
        combobox.hide();
      }
    },
    [combobox, commands, hasMatches, onSubmit],
  );

  const handleChange = useCallback(
    (event: React.ChangeEvent<HTMLTextAreaElement>) => {
      onChange(event.target.value);
      const cursor = Math.min(
        event.target.selectionStart,
        event.target.selectionEnd,
      );
      requestCommandsCompletion(event.target.value, cursor);
    },
    [onChange, requestCommandsCompletion],
  );

  const onItemClick = useCallback(
    (item: string, event: React.MouseEvent<HTMLDivElement>) => {
      event.stopPropagation();
      event.preventDefault();
      const textarea = ref.current;
      if (!textarea) return;
      modifyValueAndDispatchChange(textarea, commands, item);
    },
    [commands],
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
