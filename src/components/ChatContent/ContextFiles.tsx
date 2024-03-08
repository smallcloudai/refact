import React from "react";
import { Text, Flex } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ChatContextFile } from "../../services/refact";
import classnames from "classnames";
import { TruncateLeft } from "../Text";

export const ContextFile: React.FC<{
  name: string;
  children: string;
  className?: string;
}> = ({ name, ...props }) => {
  return (
    <Text
      size="1"
      title={props.children}
      className={classnames(styles.file, props.className)}
    >
      📎 <TruncateLeft>{name}</TruncateLeft>
    </Text>
  );
};

export const ContextFiles: React.FC<{ files: ChatContextFile[] }> = ({
  files,
}) => {
  if (files.length === 0) return null;
  return (
    <pre>
      <Flex gap="4" wrap="nowrap" direction="column" px="2">
        {files.map((file, index) => {
          const lineText =
            file.line1 && file.line2 ? `:${file.line1}-${file.line2}` : "";
          return (
            <ContextFile key={index} name={file.file_name + lineText}>
              {file.file_content}
            </ContextFile>
          );
        })}
      </Flex>
    </pre>
  );
};
