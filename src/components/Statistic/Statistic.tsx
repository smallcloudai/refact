import React, { useEffect, useState } from "react";
import { Box, Flex, Button, Heading, Responsive } from "@radix-ui/themes";
import { RefactTableData } from "../../services/refact";
import { Table } from "../Table/Table";
import { Chart } from "../Chart/Chart";
import { Spinner } from "../Spinner";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useConfig } from "../../contexts/config-context";
import { ScrollArea } from "../ScrollArea";
import { TABLE } from "../../__fixtures__";

export const Statistic: React.FC<{
  onCloseStatistic?: () => void;
  backFromChat: () => void;
}> = ({ onCloseStatistic, backFromChat }) => {
  const [isLoaded, setIsLoaded] = useState<boolean>(false);
  const [refactTable, setRefactTable] = useState<RefactTableData | null>(null);
  const { host, tabbed } = useConfig();

  const LeftRightPadding: Responsive<
    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
  > =
    host === "web"
      ? { initial: "2", xl: "9" }
      : {
          initial: "2",
          xs: "2",
          sm: "4",
          md: "8",
          lg: "8",
          xl: "9",
        };

  const TopBottomPadding: Responsive<
    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
  > = {
    initial: "5",
  };

  useEffect(() => {
    if (TABLE.data) {
      setRefactTable(JSON.parse(TABLE.data) as RefactTableData);
      setIsLoaded(true);
    }
  }, []);

  return (
    <Flex
      direction="column"
      justify="between"
      grow="1"
      pl={LeftRightPadding}
      pt={TopBottomPadding}
      pb={TopBottomPadding}
      style={{
        height: "100dvh",
      }}
    >
      {host === "vscode" && !tabbed ? (
        <Flex gap="2" pb="3">
          <Button variant="surface" onClick={backFromChat}>
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
        </Flex>
      ) : (
        <Button mr="auto" variant="outline" onClick={onCloseStatistic} mb="4">
          Back
        </Button>
      )}
      <ScrollArea scrollbars="vertical">
        <Flex
          direction="column"
          justify="between"
          grow="1"
          mr={LeftRightPadding}
          style={{
            width: "inherit",
          }}
        >
          {isLoaded ? (
            <Box
              style={{
                width: "inherit",
              }}
            >
              <Flex
                direction="column"
                style={{
                  width: "inherit",
                }}
              >
                <Heading as="h3" align="center" mb="1">
                  Statistics
                </Heading>
                {refactTable !== null && (
                  <Flex align="center" justify="center" direction="column">
                    <Table
                      refactImpactTable={refactTable.table_refact_impact.data}
                    />
                    <Chart
                      refactImpactDatesWeekly={
                        refactTable.refact_impact_dates.data.weekly
                      }
                    />
                  </Flex>
                )}
              </Flex>
            </Box>
          ) : (
            <Spinner />
          )}
        </Flex>
      </ScrollArea>
    </Flex>
  );
};
