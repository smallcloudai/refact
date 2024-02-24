import React, { useEffect, useState } from "react";
import { Box, Flex, Button, Heading, Responsive } from "@radix-ui/themes";
import { Table } from "../components/Table/Table";
import { Chart } from "../components/Chart/Chart";
import { Spinner } from "../components/Spinner";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useConfig } from "../contexts/config-context";
import { ScrollArea } from "../components/ScrollArea";
import { useEventBusForStatistic } from "../hooks";

export const Statistic: React.FC<{
  onCloseStatistic?: () => void;
}> = ({ onCloseStatistic }) => {
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const { host, tabbed } = useConfig();
  const { backFromStatistic, statisticData } = useEventBusForStatistic();
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
    if (statisticData) {
      setIsLoading(false);
    }
  }, [statisticData]);

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
          <Button variant="surface" onClick={backFromStatistic}>
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
          {isLoading ? (
            <Spinner />
          ) : (
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
                {statisticData !== null && (
                  <Flex align="center" justify="center" direction="column">
                    <Table
                      refactImpactTable={statisticData.table_refact_impact.data}
                    />
                    <Chart
                      refactImpactDatesWeekly={
                        statisticData.refact_impact_dates.data.weekly
                      }
                    />
                  </Flex>
                )}
              </Flex>
            </Box>
          )}
        </Flex>
      </ScrollArea>
    </Flex>
  );
};
