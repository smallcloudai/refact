import React from "react";
import { Button, Flex } from "@radix-ui/themes";
import { RightButton, RightButtonGroup } from "../Buttons/";
import { ScrollArea } from "../ScrollArea";
import "./highlightjs.css";
import { useConfig } from "../../contexts/config-context";

const PreTagWithButtons: React.FC<
  React.PropsWithChildren<{
    onCopyClick: () => void;
    onNewFileClick: () => void;
    onPasteClick: () => void;
    canPaste: boolean;
  }>
> = ({
  children,
  onCopyClick,
  onNewFileClick,
  onPasteClick,
  canPaste,
  ...props
}) => {
  const config = useConfig();

  return (
    <ScrollArea scrollbars="horizontal">
      <pre {...props}>
        {config.host === "web" ? (
          <RightButton onClick={onCopyClick}>Copy</RightButton>
        ) : (
          <RightButtonGroup
            direction="column"
            style={{
              position: "static",
              minHeight: "var(--space-5)",
            }}
          >
            <Flex
              gap="1"
              justify="end"
              style={{ position: "absolute", right: "var(--space-1)" }}
            >
              <Button variant="surface" size="1" onClick={onNewFileClick}>
                New File
              </Button>
              <Button size="1" variant="surface" onClick={onCopyClick}>
                Copy
              </Button>
              {canPaste && (
                <Button variant="surface" size="1" onClick={onPasteClick}>
                  Paste
                </Button>
              )}
            </Flex>
          </RightButtonGroup>
        )}
        {children}
      </pre>
    </ScrollArea>
  );
};

const PreTagWithoutButtons: React.FC<React.PropsWithChildren> = (props) => {
  return (
    <ScrollArea scrollbars="horizontal">
      <pre {...props} />
    </ScrollArea>
  );
};

export type PreTagProps = {
  onCopyClick?: () => void;
  onNewFileClick?: () => void;
  onPasteClick?: () => void;
  canPaste?: boolean;
};

export const PreTag: React.FC<React.PropsWithChildren<PreTagProps>> = ({
  onCopyClick,
  onNewFileClick,
  onPasteClick,
  canPaste,
  ...props
}) => {
  if (onCopyClick && onNewFileClick && onPasteClick) {
    return (
      <PreTagWithButtons
        {...props}
        onCopyClick={onCopyClick}
        onNewFileClick={onNewFileClick}
        onPasteClick={onPasteClick}
        canPaste={!!canPaste}
      />
    );
  }
  return <PreTagWithoutButtons {...props} />;
};
