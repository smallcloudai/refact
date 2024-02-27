import React from "react";
import { Flex } from "@radix-ui/themes";
import { TruncateLeft, Text } from "../Text";
import type { ChatContextFile } from "../../events";

export type FileListProps = { files: ChatContextFile[] };
export const FileList: React.FC<FileListProps> = ({ files }) => {
  return (
    <Flex>
      {files.map((file, i) => {
        const key = `${file.file_name}:${file.line1}-${file.line2}:${i}`;
        return (
          <Text key={key} size="2" title={file.file_content}>
            📎 <TruncateLeft>{file.file_name}</TruncateLeft>
          </Text>
        );
      })}
    </Flex>
  );
};
