import React from "react";
import type { ChatState } from "../../hooks/useEventBusForChat";
import type { Snippet } from "../../events";
import { Collapsible } from "../Collapsible";
import { Checkbox, Box, Grid, Text } from "@radix-ui/themes";

const symbolsRegExp = /(@symbols-at)([\s]+)([^\s]*)/g;
const workspaceWithFileRegExp = /@workspace ([^\s])+/g;
const workspaceWithoutFileRegExp = /@workspace(\s?)*\n/g;
const markdownRegexp = /(?:\n)?^```[\w]*\n(.|\n)*\n```/gm;

export type CursorPosition = {
  start: number;
  end: number;
};

export const ChatControls: React.FC<{
  value: string;
  activeFile: ChatState["active_file"];
  snippet: Snippet;
  onChange: (value: string) => void;
  cursorPosition?: CursorPosition | null;
}> = ({
  value,
  activeFile: _activeFile,
  onChange,
  snippet,
  cursorPosition,
}) => {
  const searchActive =
    !workspaceWithFileRegExp.test(value) &&
    workspaceWithoutFileRegExp.test(value);

  const handleSearchChange = (checked: boolean) => {
    if (!checked) {
      const nextValue = value.replace(workspaceWithoutFileRegExp, "");
      onChange(nextValue);
    } else if (cursorPosition) {
      const startPosition = Math.min(cursorPosition.start, cursorPosition.end);
      const endPosition = Math.max(cursorPosition.start, cursorPosition.end);
      const start = value.substring(0, startPosition);
      const end = value.substring(endPosition);
      const nextValue = `${start}${start.length ? "\n" : ""}@workspace\n${end}`;
      onChange(nextValue);
    } else {
      const nextValue = value.length
        ? value + "\n" + "@workspace\n"
        : "@workspace\n";
      onChange(nextValue);
    }
  };

  const lookupActive = symbolsRegExp.test(value);

  const handleLookupChange = (checked: boolean) => {
    if (!checked) {
      const nextValue = value.replace(symbolsRegExp, "");
      onChange(nextValue);
    } else if (cursorPosition) {
      const startPosition = Math.min(cursorPosition.start, cursorPosition.end);
      const endPosition = Math.max(cursorPosition.start, cursorPosition.end);
      const start = value.substring(0, startPosition);
      const end = value.substring(endPosition);
      const nextValue = `${start}${start.length ? "\n" : ""}@symbols-at ${
        end.length ? "\n" : ""
      }${end}`;
      onChange(nextValue);
    } else {
      const nextValue = `${value.length ? value + "\n" : ""}@symbols-at `;
      onChange(nextValue);
    }
  };

  const hasMarkdown = markdownRegexp.test(value);
  const handleLinesActive = (checked: boolean) => {
    // TODO: this might need the current selected snippet to be reimplemented
    const markdown = "```" + snippet.language + "\n" + snippet.code + "\n```";
    if (!checked && !cursorPosition) {
      const nextValue = value.length ? value + "\n" + markdown : markdown;
      onChange(nextValue);
    } else if (!checked && cursorPosition) {
      const startPosition = Math.min(cursorPosition.start, cursorPosition.end);
      const endPosition = Math.max(cursorPosition.start, cursorPosition.end);
      const start = value.substring(0, startPosition);
      const end = value.substring(endPosition);
      const nextValue = `${start}${start.length ? "\n" : ""}${markdown}${end}`;
      onChange(nextValue);
    } else if (!checked) {
      const nextValue = value.length ? value + "\n" + markdown : markdown;
      onChange(nextValue);
    } else {
      const nextValue = value.replace(markdownRegexp, "");
      onChange(nextValue);
    }
  };

  return (
    <Box pt="4" pb="4" pl="2">
      <Collapsible title="Advanced: ">
        <Grid columns="2" width="auto" gap="2">
          <Text size="2">
            <Checkbox
              size="1"
              name="search_workspace"
              checked={searchActive}
              onCheckedChange={handleSearchChange}
            />{" "}
            Search workspace
          </Text>

          <Text size="2">
            <Checkbox
              size="1"
              name="lookup_symbols"
              checked={lookupActive}
              onCheckedChange={handleLookupChange}
            />{" "}
            Lookup symbols
          </Text>

          <Text size="2">
            <Checkbox
              size="1"
              name="selected_lines"
              checked={hasMarkdown}
              onCheckedChange={handleLinesActive}
            />{" "}
            Selected lines
          </Text>
        </Grid>
      </Collapsible>
    </Box>
  );
};
