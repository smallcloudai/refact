import React from "react";
import { Box, Button } from "@radix-ui/themes";
import { Text, TruncateLeft } from "../Text";
import { ChatContextFile } from "../../events";
import styles from "./ChatForm.module.css";

export const FilesPreview: React.FC<{
  files: ChatContextFile[];
  onRemovePreviewFile: (name: string) => void;
}> = ({ files, onRemovePreviewFile }) => {
  if (files.length === 0) return null;
  return (
    <Box p="2" pb="0">
      {files.map((file, i) => {
        const lineText =
          file.line1 && file.line2 ? `:${file.line1}-${file.line2}` : "";
        return (
          <pre key={file.file_name + i} className={styles.file}>
            <Text
              size="1"
              title={file.file_content}
              className={styles.file_name}
            >
              <Button
                onClick={(event) => {
                  event.preventDefault();
                  onRemovePreviewFile(file.file_name);
                }}
                variant="ghost"
                className={styles.removeFileButton}
              >
                📎
              </Button>
              <TruncateLeft>
                {file.file_name}
                {lineText}
              </TruncateLeft>
            </Text>
          </pre>
        );
      })}
    </Box>
  );
};
