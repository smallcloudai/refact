import React from "react";
import { Flex, Text } from "@radix-ui/themes";
import { ContextQueries } from "../../events";
import { TruncateLeft } from "../Text";

export const SymbolList: React.FC<{
  symbols: ContextQueries;
}> = ({ symbols }) => {
  return (
    <Flex direction="column">
      {symbols.map(({ symbol, from }, index) => {
        const key = `${symbol}-${from}-${index}`;
        return (
          <Text
            key={key}
            title={from}
            size="2"
            as="div"
            style={{ display: "flex" }}
          >
            🔎
            <TruncateLeft>{symbol}</TruncateLeft>
          </Text>
        );
      })}
    </Flex>
  );
};
