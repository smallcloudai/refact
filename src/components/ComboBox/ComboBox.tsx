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
import { getAnchorRect, type AnchorRect } from "./utils";
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

export const ComboBox: React.FC<{
  commands: string[];
  onChange: React.Dispatch<React.SetStateAction<string>>;
  value: string;
  onSubmit: React.KeyboardEventHandler<HTMLTextAreaElement>;
  placeholder?: string;
  render: (props: TextAreaProps) => React.ReactElement;
}> = ({ commands, onSubmit, placeholder, onChange, value, render }) => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [trigger, setTrigger] = React.useState<string>("");

  const combobox = useComboboxStore({
    defaultOpen: false,
    placement: "top-start",
  });

  const matches = matchSorter(commands, trigger, {
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
  };

  const onKeyUp = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const state = combobox.getState();
    const tabOrEnter = event.key === "Tab" || event.key === "Enter";
    if (state.open && tabOrEnter && state.activeValue) {
      event.preventDefault();
      const newInput = value.replace(trigger, state.activeValue + " ");
      combobox.setValue(newInput);
      onChange(newInput);
      combobox.hide();
    } else if (event.key === "Enter") {
      onSubmit(event);
    }
  };

  const handleChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
    const maybeCommand = event.target.value.startsWith("@")
      ? event.target.value.split(/\s/)[0]
      : "";

    if (maybeCommand && event.target.selectionEnd <= maybeCommand.length) {
      setTrigger(maybeCommand);
      combobox.show();
    } else {
      setTrigger("");
      combobox.hide();
    }
    onChange(event.target.value);
    combobox.setValue(trigger);
  };

  const onItemClick = (item: string) => {
    const textarea = ref.current;
    if (!textarea) return;
    onChange((prevValue) => prevValue.replace(trigger, item + " "));
    setTrigger(() => "");
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
          return getAnchorRect(textarea, matches);
        }}
      >
        {matches.map((item, index) => (
          <Item
            key={item + "-" + index}
            value={item}
            onClick={(event) => {
              event.stopPropagation();
              event.preventDefault();
              onItemClick(item);
            }}
          >
            {item}
          </Item>
        ))}
      </Popover>
    </>
  );
};
