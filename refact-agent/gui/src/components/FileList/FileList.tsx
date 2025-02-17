import React from "react";
import { Flex } from "@radix-ui/themes";
import { TruncateLeft, Text } from "../Text";
import type { ChatContextFile } from "../../services/refact";
import styles from "./file-list.module.css";

export type FileListProps = { files: ChatContextFile[] };
export const FileList: React.FC<FileListProps> = ({ files }) => {
  return (
    <Flex direction="column">
      {files.map((file, i) => {
        const name = `${file.file_name}:${file.line1}-${file.line2}`;
        const key = `${name}--${i}`;
        return (
          <Text
            as="div"
            key={key}
            size="2"
            title={file.file_content}
            className={styles.file}
          >
            📎&nbsp;<TruncateLeft>{name}</TruncateLeft>
          </Text>
        );
      })}
    </Flex>
  );
};
