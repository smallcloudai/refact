import React from "react";
import { Flex, Text, Box } from "@radix-ui/themes";
import { Buckets } from "../../events";
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

export type SymbolListProps = {
  symbols: {
    bucket_declarations: Buckets;
    bucket_usage_of_same_stuff: Buckets;
    bucket_high_overlap: Buckets;
    cursor_symbols: Buckets;
  };
};
export const SymbolList: React.FC<SymbolListProps> = ({ symbols }) => {
  const declarations = symbols.bucket_declarations;
  const usages = symbols.bucket_usage_of_same_stuff;
  const overLap = symbols.bucket_high_overlap;
  const cursorSymbols = symbols.cursor_symbols;

  return (
    <Flex direction="column" gap="4">
      <Collapsible defaultOpen title={`Declarations: ${declarations.length}`}>
        {declarations.map(({ name }, i) => {
          const key = `declaration-${i}`;
          return <SymbolText key={key}>{name}</SymbolText>;
        })}
      </Collapsible>

      <Collapsible title={`Cursor Symbols: ${cursorSymbols.length}`}>
        {cursorSymbols.map(({ name }, i) => {
          const key = `cursor-symbols-${i}`;
          return <SymbolText key={key}>{name}</SymbolText>;
        })}
      </Collapsible>

      <Collapsible title={`Usages: ${usages.length}`}>
        {usages.map(({ name }, i) => {
          const key = `usages-${i}`;
          return <SymbolText key={key}>{name}</SymbolText>;
        })}
      </Collapsible>

      <Collapsible title={`High Overlap: ${overLap.length}`}>
        {overLap.map(({ name }, i) => {
          const key = `high-overlap-${i}`;
          return <SymbolText key={key}>{name}</SymbolText>;
        })}
      </Collapsible>
    </Flex>
  );
};
