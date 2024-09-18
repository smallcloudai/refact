import React from "react";
import { Flex, Container, Text } from "@radix-ui/themes";
import { FileList } from "../FileList";
import { SymbolList } from "./SymoblList";

import type { FIMContext } from "../../services/refact";

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

      <Container>
        <SymbolList symbols={data} />
      </Container>
    </Flex>
  );
};
