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

const INTERACTIVE_BUTTONS_CONTAINER_HEIGHT = 30;
const MINIMAL_TEXTAREA_HEIGHT = 95;
const VIEWPORT_HEIGHT_THRESHOLD = 0.9;
const PADDING_TOP_EXPANDED = 8;

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
        const eventKey = event.key.toLowerCase();

        if (isMod && eventKey === "z" && !event.shiftKey) {
          event.preventDefault();
          undoRedo.undo();
          setCallChange(true);
        }

        if (isMod && eventKey === "z" && event.shiftKey) {
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
        const textArea = innerRef.current;
        const parentElement = textArea.parentElement;

        textArea.style.height = "1px";

        const contentHeight = value
          ? 2 + INTERACTIVE_BUTTONS_CONTAINER_HEIGHT + textArea.scrollHeight
          : MINIMAL_TEXTAREA_HEIGHT;

        textArea.style.height = contentHeight + "px";

        if (parentElement) {
          const shouldExpandPaddings =
            textArea.scrollHeight >
            (window.innerHeight / 2) * VIEWPORT_HEIGHT_THRESHOLD;

          const updatedPaddingBottom = shouldExpandPaddings
            ? INTERACTIVE_BUTTONS_CONTAINER_HEIGHT * 1.5
            : 0;

          const updatedPaddingTop = shouldExpandPaddings
            ? PADDING_TOP_EXPANDED
            : 0;

          parentElement.style.paddingBottom = updatedPaddingBottom + "px";
          parentElement.style.paddingTop = updatedPaddingTop + "px";
        }
        onTextAreaHeightChange && onTextAreaHeightChange(textArea.scrollHeight);
      }
    }, [innerRef.current?.value, value, onTextAreaHeightChange]);

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
