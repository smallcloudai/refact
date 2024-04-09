import React from "react";
import { Flex, Text, Box } from "@radix-ui/themes";
import { ContextQueries } from "../../events";
import { TruncateLeft } from "../Text";
import { Collapsible } from "../Collapsible";
import styles from "./fim.module.css";

const SymbolText: React.FC<{
  children: React.ReactNode;
  title?: string;
}> = ({ children, title }) => {
  return (
    <Box p="1">
      <Text title={title} size="2" as="div" style={{ display: "flex" }}>
        🔎
        <TruncateLeft className={styles.symbol}>{children}</TruncateLeft>
      </Text>
    </Box>
  );
};

export const SymbolList: React.FC<{
  symbols?: ContextQueries;
}> = ({ symbols = [] }) => {
  const declarations = symbols.filter(({ from }) => from === "declarations");
  const cursorSymbols = symbols.filter(({ from }) => from === "cursor_symbols");
  const usages = symbols.filter(({ from }) => from === "usages");

  return (
    <Flex direction="column" gap="4">
      <Collapsible defaultOpen title={`Declarations: ${declarations.length}`}>
        {declarations.map(({ symbol }, i) => {
          const key = `declaration-${i}`;
          return <SymbolText key={key}>{symbol}</SymbolText>;
        })}
      </Collapsible>

      <Collapsible title={`Cursor Symbols: ${cursorSymbols.length}`}>
        {cursorSymbols.map(({ symbol }, i, arr) => {
          const key = `cursor-symbols-${i}`;
          return (
            <SymbolText key={key}>
              {symbol}&nbsp;{arr.length}
            </SymbolText>
          );
        })}
      </Collapsible>

      <Collapsible title={`Usages: ${usages.length}`}>
        {usages.map(({ symbol }, i) => {
          const key = `usages-${i}`;
          return <SymbolText key={key}>{symbol}</SymbolText>;
        })}
      </Collapsible>
    </Flex>
  );
};
