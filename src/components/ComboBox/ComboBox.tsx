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
import { TextArea } from "../TextArea";
import { ScrollArea } from "../ScrollArea";
import { Button } from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./ComboBox.module.css";

const Item: React.FC<{
  onClick: React.MouseEventHandler<HTMLDivElement>;
  value: string;
  children: React.ReactNode;
}> = ({ children, value, onClick }) => {
  return (
    <Button className={styles.item} variant="ghost" asChild>
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

export const ComboBox = () => {
  const commands = [
    "@workspace",
    "@help",
    "@list",
    "@web",
    "@database",
    "@?",
    "@longlonglonglong",
    "@refactor",
    "@test",
    "@marc",
    "@Apple",
    "@Banana",
    "@Carrot",
    "@Dill",
    "@Elderberries",
    "@Figs",
    "@Grapes",
    "@Honeydew",
    "@Iced melon",
    "@Jackfruit",
    "@Kale",
    "@Lettuce",
    "@Mango",
    "@Nectarines",
    "@Oranges",
    "@Pineapple",
    "@Quince",
    "@Raspberries",
    "@Strawberries",
    "@Turnips",
    "@Ugli fruit",
    "@Vanilla beans",
    "@Watermelon",
    "@Xigua",
    "@Yuzu",
    "@Zucchini",
  ];
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [value, setValue] = React.useState("");
  const [trigger, setTrigger] = React.useState<string>("");

  const combobox = useComboboxStore();

  const matches = matchSorter(commands, trigger, {
    baseSort: (a, b) => (a.index < b.index ? -1 : 1),
  });

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
      setValue(newInput);
      combobox.setValue(newInput);
      combobox.hide();
    }
  };

  const onChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
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
    setValue(event.target.value);
    combobox.setValue(trigger);
  };

  const onItemClick = (item: string) => {
    const textarea = ref.current;
    if (!textarea) return;
    setValue((prevValue) => prevValue.replace(trigger, item + " "));
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
        render={
          <TextArea
            ref={ref}
            rows={5} // TODO: remove this later
            placeholder="Type @ for commands"
            onScroll={combobox.render}
            onPointerDown={combobox.hide}
            onChange={onChange}
            onKeyDown={onKeyDown}
          />
        }
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
      {/* <Box asChild>
        <ComboboxPopover
          className={styles.popover2}
          store={combobox}
          hidden={!hasMatches}
          unmountOnHide
          fitViewport
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
        </ComboboxPopover>
      </Box> */}
    </>
  );
};
