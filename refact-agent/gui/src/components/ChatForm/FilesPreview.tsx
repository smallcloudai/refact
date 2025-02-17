import React from "react";
import { Box } from "@radix-ui/themes";
import { Text, TruncateLeft } from "../Text";
import { ChatContextFile } from "../../services/refact";
import styles from "./ChatForm.module.css";

const FileNameAndContent: React.FC<{
  title: string;
  children: React.ReactNode;
}> = ({ title, children }) => {
  return (
    <pre className={styles.file}>
      <Text size="1" title={title} className={styles.file_name}>
        {children}
      </Text>
    </pre>
  );
};

const Preview: React.FC<{ file: string | ChatContextFile }> = ({ file }) => {
  if (typeof file === "string") {
    return (
      <FileNameAndContent title={file}>
        📄&nbsp;<TruncateLeft>plain text</TruncateLeft>
      </FileNameAndContent>
    );
  }

  const lineText =
    file.line1 !== 0 && file.line2 !== 0 && `:${file.line1}-${file.line2}`;

  return (
    <FileNameAndContent title={file.file_content}>
      📎&nbsp;
      <TruncateLeft>
        {file.file_name}
        {lineText}
      </TruncateLeft>
    </FileNameAndContent>
  );
};

export const FilesPreview: React.FC<{
  files: (ChatContextFile | string)[];
}> = ({ files }) => {
  if (files.length === 0) return null;
  return (
    <Box p="2" pb="0">
      {files.map((file, i) => {
        const key =
          typeof file === "string"
            ? `plain-text-preview-${i}`
            : `file-preview-${i}-${file.file_name}`;
        return <Preview key={key} file={file} />;
      })}
    </Box>
  );
};
