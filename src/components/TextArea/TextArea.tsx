import React, {
  useEffect,
  useImperativeHandle,
  useRef,
  useLayoutEffect,
  useCallback,
} from "react";
import { TextArea as RadixTextArea } from "@radix-ui/themes";
import classNames from "classnames";
import { useUndoRedo } from "../../hooks";
import { createSyntheticEvent } from "../../utils/createSyntheticEvent";
import styles from "./TextArea.module.css";

export type TextAreaProps = React.ComponentProps<typeof RadixTextArea> &
  React.JSX.IntrinsicElements["textarea"] & {
    onTextAreaHeightChange?: (scrollHeight: number) => void;
    onChange: (event: React.ChangeEvent<HTMLTextAreaElement>) => void;
  };

export const TextArea = React.forwardRef<HTMLTextAreaElement, TextAreaProps>(
  ({ onTextAreaHeightChange, value, onKeyDown, onChange, ...props }, ref) => {
    const [callChange, setCallChange] = React.useState(true);
    const innerRef = useRef<HTMLTextAreaElement>(null);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    useImperativeHandle(ref, () => innerRef.current!, []);
    const undoRedo = useUndoRedo(value);

    const handleKeyDown = useCallback(
      (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
        const isMod = event.metaKey || event.ctrlKey;
        if (isMod && event.key === "z" && !event.shiftKey) {
          event.preventDefault();
          undoRedo.undo();
          setCallChange(true);
        }

        if (isMod && event.key === "z" && event.shiftKey) {
          event.preventDefault();
          undoRedo.redo();
          setCallChange(true);
        }

        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
        }

        onKeyDown && onKeyDown(event);
      },
      [onKeyDown, undoRedo],
    );

    const handleChange = useCallback(
      (event: React.ChangeEvent<HTMLTextAreaElement>) => {
        onChange(event);
      },
      [onChange],
    );

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
      if (value !== undoRedo.state) {
        undoRedo.setState(value);
      }
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [value]);

    useLayoutEffect(() => {
      if (innerRef.current && callChange && undoRedo.state !== value) {
        const e = new Event("change", { bubbles: true });
        Object.defineProperty(e, "target", {
          writable: true,
          value: {
            ...innerRef.current,
            value: undoRedo.state,
          },
        });

        Object.defineProperty(e, "currentTarget", {
          writable: true,
          value: {
            ...innerRef.current,
            value: undoRedo.state,
          },
        });
        const syntheticEvent = createSyntheticEvent(
          e,
        ) as React.ChangeEvent<HTMLTextAreaElement>;

        queueMicrotask(() => onChange(syntheticEvent));
        setCallChange(false);
      } else if (callChange) {
        setCallChange(false);
      }
    }, [callChange, undoRedo.state, onChange, value]);

    return (
      <RadixTextArea
        {...props}
        value={value}
        onKeyDown={handleKeyDown}
        onChange={handleChange}
        className={classNames(styles.textarea, props.className)}
        ref={innerRef}
      />
    );
  },
);

TextArea.displayName = "TextArea";
