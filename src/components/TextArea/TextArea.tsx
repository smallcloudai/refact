import React, { useEffect, useImperativeHandle, useRef } from "react";
import { TextArea as RadixTextArea } from "@radix-ui/themes";
import classNames from "classnames";
import { useUndoRedo } from "../../hooks";
import styles from "./TextArea.module.css";

export type TextAreaProps = React.ComponentProps<typeof RadixTextArea> &
  React.JSX.IntrinsicElements["textarea"] & {
    onTextAreaHeightChange?: (scrollHeight: number) => void;
    onChange: (event: React.ChangeEvent<HTMLTextAreaElement>) => void;
    onSubmit: React.FormEventHandler<HTMLTextAreaElement>;
  };

export const TextArea = React.forwardRef<HTMLTextAreaElement, TextAreaProps>(
  (
    { onTextAreaHeightChange, value, onKeyDown, onChange, onSubmit, ...props },
    ref,
  ) => {
    const innerRef = useRef<HTMLTextAreaElement>(null);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    useImperativeHandle(ref, () => innerRef.current!, []);
    const undoRedo = useUndoRedo(value);

    const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      const isMod = event.metaKey || event.ctrlKey;
      if (isMod && event.key === "z" && !event.shiftKey) {
        event.preventDefault();
        undoRedo.undo();
      }

      if (isMod && event.key === "z" && event.shiftKey) {
        event.preventDefault();
        undoRedo.redo();
      }

      onKeyDown && onKeyDown(event);
    };

    const handleChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
      onChange(event);
    };

    const handleSubmit = (event: React.FormEvent<HTMLTextAreaElement>) => {
      onSubmit(event);
      undoRedo.reset("");
    };

    useEffect(() => {
      if (innerRef.current) {
        innerRef.current.style.height = "1px";
        innerRef.current.style.height =
          2 + innerRef.current.scrollHeight + "px";
        onTextAreaHeightChange &&
          onTextAreaHeightChange(innerRef.current.scrollHeight);
      }
    }, [innerRef.current?.value, onTextAreaHeightChange]);

    useEffect(() => {
      undoRedo.setState(value);
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [value]);

    return (
      <RadixTextArea
        {...props}
        value={undoRedo.state}
        onKeyDown={handleKeyDown}
        onChange={handleChange}
        className={classNames(styles.textarea, props.className)}
        onSubmit={handleSubmit}
        ref={innerRef}
      />
    );
  },
);

TextArea.displayName = "TextArea";
