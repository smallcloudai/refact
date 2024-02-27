import React from "react";
import { Heading, Box } from "@radix-ui/themes";
import type { StatisticState } from "../../hooks";
import { FileList } from "../FileList";
import { Callout } from "../Callout";
import { Spinner } from "../Spinner";

export type FimFilesProps = { fimFiles: StatisticState["fill_in_the_middle"] };
export const FimFiles: React.FC<FimFilesProps> = (props) => {
  return (
    <Box>
      <Heading size="3">Fill In The Middle Files</Heading>
      <DisplayMessageOrFiles {...props.fimFiles} />
    </Box>
  );
};

const DisplayMessageOrFiles: React.FC<FimFilesProps["fimFiles"]> = (props) => {
  if (props.files.length > 0) {
    return <FileList files={props.files} />;
  }

  if (props.fetching) {
    return <Spinner />;
  }

  if (props.error) {
    return <Callout type="error">{props.error}</Callout>;
  }

  return <Callout type="info">No files found</Callout>;
};
