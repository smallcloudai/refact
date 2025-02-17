import React from "react";
import "./highlightjs.css";
import { Flex, Button } from "@radix-ui/themes";
import { useConfig } from "../../hooks";
import { RightButtonGroup, RightButton } from "../Buttons";

const PreTagWithButtons: React.FC<
  React.PropsWithChildren<{
    onCopyClick: () => void;
  }>
> = ({ children, onCopyClick, ...props }) => {
  const config = useConfig();

  return (
    <pre {...props}>
      {config.host === "web" ? (
        <RightButtonGroup
          direction="column"
          style={{
            position: "static",
            minHeight: "var(--space-6)",
          }}
        >
          <Flex
            gap="1"
            justify="end"
            style={{ position: "absolute", right: "0" }}
            pr="2"
            pt="1"
          >
            <RightButton onClick={onCopyClick}>Copy</RightButton>
          </Flex>
        </RightButtonGroup>
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
            style={{ position: "absolute", right: "0" }}
            pr="2"
          >
            <Button size="1" variant="surface" onClick={onCopyClick}>
              ⿻ Copy
            </Button>
          </Flex>
        </RightButtonGroup>
      )}
      {children}
    </pre>
  );
};

export type PreTagProps = {
  onCopyClick?: () => void;
};

export const PreTag: React.FC<React.PropsWithChildren<PreTagProps>> = (
  props,
) => {
  if (props.onCopyClick) {
    return <PreTagWithButtons {...props} onCopyClick={props.onCopyClick} />;
  }
  return <pre {...props} />;
};
