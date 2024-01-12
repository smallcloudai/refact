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
      <ComboboxItem value={value} onClick={onClick} focusOnHover>
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
        <ScrollArea scrollbars="vertical" className={styles.popover__scroll}>
          <Box p="1" style={{ overflowY: "hidden" }}>
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
  onKeyUp: React.KeyboardEventHandler<HTMLTextAreaElement>;
  placeholder?: string;
  render: (props: TextAreaProps) => React.ReactElement;
}> = ({ commands, onKeyUp, placeholder, onChange, value, render }) => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [trigger, setTrigger] = React.useState<string>("");

  const combobox = useComboboxStore();

  const matches = matchSorter(commands, trigger, {
    baseSort: (a, b) => (a.index < b.index ? -1 : 1),
  }); //.slice(0, 10);

  const hasMatches = !!matches.length;

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
  }, [combobox, hasMatches]);

  React.useEffect(() => {
    combobox.render();
  }, [combobox, value]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // TODO: pressing enter should submit the form
    // TODO: shift+enter should create a new line
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      combobox.hide();
    }
    if (trigger && matches.length && event.key === "Tab") {
      event.preventDefault();
      const match = matches[0];
      const newInput = value.replace(trigger, match);
      combobox.setValue(newInput);
      onChange(newInput);
      combobox.hide();
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
        setValueOnChange={false}
        render={render({
          ref,
          placeholder,
          onScroll: combobox.render,
          onPointerDown: combobox.hide,
          onChange: handleChange,
          onKeyDown: onKeyDown,
          onKeyUp: onKeyUp,
        })}
      />
      <Popover
        store={combobox}
        hidden={!hasMatches}
        getAnchorRect={() => {
          const textarea = ref.current;
          if (!textarea) return null;
          return getAnchorRect(textarea, commands);
        }}
      >
        {matches.map((item, index) => (
          <Item
            key={item + "-" + index}
            value={item}
            onClick={() => onItemClick(item)}
          >
            {item}
          </Item>
        ))}
      </Popover>
    </>
  );
};
