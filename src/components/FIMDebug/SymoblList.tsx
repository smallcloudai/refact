import React from "react";
import { Flex, Text } from "@radix-ui/themes";
import { ContextQueries } from "../../events";
import { TruncateLeft } from "../Text";
import { Collapsible } from "../Collapsible";
import styles from "./fim.module.css";

const SymbolText: React.FC<{
  children: React.ReactNode;
  title?: string;
}> = ({ children, title }) => {
  return (
    <Text title={title} size="2" as="div" style={{ display: "flex" }}>
      🔎
      <TruncateLeft className={styles.symbol}>{children}</TruncateLeft>
    </Text>
  );
};

export const SymbolList: React.FC<{
  symbols: ContextQueries;
}> = ({ symbols }) => {
  const declarations = symbols.filter(({ from }) => from === "declarations");
  const cursorSymbols = symbols.filter(({ from }) => from === "cursor_symbols");
  const usages = symbols.filter(({ from }) => from === "usages");
  const matchedByNameSymbols = symbols.filter(
    ({ from }) => from === "matched_by_name_symbols",
  );

  return (
    <Flex direction="column" gap="4">
      {declarations.length > 0 && (
        <Collapsible defaultOpen={true} title="Declarations">
          {declarations.map(({ symbol }, i) => {
            const key = `declaration-${i}`;
            return <SymbolText key={key}>{symbol}</SymbolText>;
          })}
        </Collapsible>
      )}

      {matchedByNameSymbols.length > 0 && (
        <Collapsible title="Matched by name">
          {matchedByNameSymbols.map(({ symbol }, i) => {
            const key = `matched-by-name-${i}`;
            return <SymbolText key={key}>{symbol}</SymbolText>;
          })}
        </Collapsible>
      )}

      {usages.length > 0 && (
        <Collapsible title="Usages">
          {usages.map(({ symbol }, i) => {
            const key = `usages-${i}`;
            return <SymbolText key={key}>{symbol}</SymbolText>;
          })}
        </Collapsible>
      )}

      {cursorSymbols.length > 0 && (
        <Collapsible title="Cursor Symbols">
          {cursorSymbols.map(({ symbol }, i) => {
            const key = `cursor-symbols-${i}`;
            return <SymbolText key={key}>{symbol}</SymbolText>;
          })}
        </Collapsible>
      )}
    </Flex>
  );
};
