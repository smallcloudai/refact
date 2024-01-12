import React from "react";
import {
  useComboboxStore,
  Combobox,
  ComboboxPopover,
  ComboboxItem,
} from "@ariakit/react";
import { matchSorter } from "match-sorter";
import { getAnchorRect } from "./utils";
import { TextArea } from "../TextArea";

export const ComboBox = () => {
  const commands = ["@workspace", "@help", "@list"];
  const ref = React.useRef<HTMLTextAreaElement>(null);
  const [value, setValue] = React.useState("");
  const [trigger, setTrigger] = React.useState<string>("");

  const combobox = useComboboxStore();

  const matches = matchSorter(commands, trigger, {
    baseSort: (a, b) => (a.index < b.index ? -1 : 1),
  }).slice(0, 10);

  const hasMatches = !!matches.length;

  React.useLayoutEffect(() => {
    combobox.setOpen(hasMatches);
  }, [combobox, hasMatches]);

  React.useEffect(() => {
    combobox.render();
  }, [combobox, value]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
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
            rows={5}
            placeholder="Type @ for commands"
            onScroll={combobox.render}
            onPointerDown={combobox.hide}
            onChange={onChange}
            onKeyDown={onKeyDown}
          />
        }
      />
      <ComboboxPopover
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
        {matches.map((item) => (
          <ComboboxItem
            key={item}
            value={item}
            focusOnHover
            onClick={() => onItemClick(item)}
          >
            {item}
          </ComboboxItem>
        ))}
      </ComboboxPopover>
    </>
  );
};
