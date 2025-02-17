import React from "react";
import { Box, Flex, Heading } from "@radix-ui/themes";
import { Table } from "../Table/Table";
import { Chart } from "../Chart/Chart";
import { StatisticData } from "../../services/refact";
import { Spinner } from "../Spinner";
import { ErrorCallout } from "../Callout";

export const StatisticView: React.FC<{
  statisticData?: StatisticData;
  isLoading: boolean;
  error?: string;
}> = ({ statisticData, isLoading, error }) => {
  if (isLoading || !statisticData) {
    return <Spinner spinning />;
  }

  if (error ?? !statisticData) {
    return <ErrorCallout>{error}</ErrorCallout>;
  }
  return (
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
        <Heading as="h3" align="center" mb="5">
          Statistics
        </Heading>
        <Flex align="center" justify="center" direction="column">
          <Table refactImpactTable={statisticData.table_refact_impact.data} />
          <Chart
            refactImpactDatesWeekly={Object.fromEntries(
              Object.entries(
                statisticData.refact_impact_dates.data.weekly,
              ).sort(
                ([a], [b]) => new Date(a).getTime() - new Date(b).getTime(),
              ),
            )}
          />
        </Flex>
      </Flex>
    </Box>
  );
};
