import React from "react";
import { Text, Flex } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ChatContextFile } from "../../services/refact";
import classnames from "classnames";

export const ContextFile: React.FC<{
  name: string;
  children: string;
  className?: string;
}> = ({ name, ...props }) => {
  return (
    <Text
      size="2"
      title={props.children}
      className={classnames(styles.file, props.className)}
    >
      📎 {name}
    </Text>
  );
};

export const ContextFiles: React.FC<{ files: ChatContextFile[] }> = ({
  files,
}) => {
  return (
    <pre>
      <Flex gap="4" wrap="wrap">
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
