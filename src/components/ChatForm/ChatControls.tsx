import React from "react";
import type { ChatState } from "../../hooks/useEventBusForChat";
import type { Snippet } from "../../events";
import { Collapsible } from "../Collapsible";
import { Checkbox, Box, Grid, Text } from "@radix-ui/themes";

const symbolsRegExp = /@symbols-at\s+([^\s]+)/g;
const workspaceWithFileRegExp = /@workspace ([^\s])+/g;
const workspaceWithoutFileRegExp = /@workspace[\s]*\n/g;

export const ChatControls: React.FC<{
  value: string;
  activeFile: ChatState["active_file"];
  snippet: Snippet;
  onChange: (value: string) => void;
}> = ({ value, activeFile, onChange, snippet }) => {
  const searchActive =
    !workspaceWithFileRegExp.test(value) &&
    workspaceWithoutFileRegExp.test(value);

  const handleSearchChange = (checked: boolean) => {
    if (!checked) {
      const nextValue = value.replace(workspaceWithoutFileRegExp, "");
      onChange(nextValue);
    }
    // TODO: handle case when the user checks this checkbox
  };
  // regexp
  const lookupActive = symbolsRegExp.test(value);

  const handleLookupChange = (checked: boolean) => {
    if (!checked) {
      const nextValue = value.replace(symbolsRegExp, "");
      onChange(nextValue);
    }
  };

  const linesActive = activeFile.can_paste && snippet.code ? true : false;
  const handleLinesActive = (checked: boolean) => {
    if (!checked) {
      const nextValue =
        "```" + snippet.language + "\n" + snippet.code + "\n```";
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
              disabled={!searchActive}
              onCheckedChange={handleSearchChange}
            />{" "}
            Search workspace
          </Text>

          <Text size="2">
            <Checkbox
              size="1"
              name="lookup_symbols"
              checked={lookupActive}
              disabled={!lookupActive}
              onCheckedChange={handleLookupChange}
            />{" "}
            Lookup symbols
          </Text>

          <Text size="2">
            <Checkbox
              size="1"
              name="selected_lines"
              checked={linesActive}
              disabled={!linesActive}
              onCheckedChange={handleLinesActive}
            />{" "}
            Selected lines
          </Text>
        </Grid>
      </Collapsible>
    </Box>
  );
};
