import React from "react";
import { Heading, Flex, Container, Text } from "@radix-ui/themes";
import { FileList } from "../FileList";
import { SymbolList } from "./SymoblList";

import type { FIMContext } from "../../events";

export type SearchContextProps = {
  data: FIMContext;
};

export const SearchContext: React.FC<SearchContextProps> = ({ data }) => {
  return (
    <Flex direction="column">
      <Container py="3">
        {data.attached_files && data.attached_files.length > 0 ? (
          <FileList files={data.attached_files} />
        ) : (
          <Text wrap="nowrap" style={{ overflow: "hidden" }} size="2">
            No Context files attached
          </Text>
        )}
      </Container>

      <Container py="3">
        <Heading as="h4" size="2" mb="4">
          Look up symbols
        </Heading>
        <SymbolList symbols={data} />
      </Container>
    </Flex>
  );
};
