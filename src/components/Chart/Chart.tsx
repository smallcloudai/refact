import React from "react";
import { Box, Text } from "@radix-ui/themes";
import {
  RefactTableData,
  RefactTableImpactDatesRow,
} from "../../services/refact";
import ReactEcharts from "echarts-for-react";
import { Spinner } from "../Spinner";

export const Chart: React.FC<{
  refactTable: RefactTableData | null;
}> = ({ refactTable }) => {
  if (refactTable === null) {
    return <Spinner />;
  }
  const refactImpactDatesWeekly: RefactTableImpactDatesRow[] =
    refactTable.refact_impact_dates.data.weekly;
  const dates: string[] = Object.keys(refactImpactDatesWeekly).map((date) => {
    return new Date(date).toLocaleString("en-US", {
      month: "short",
      day: "numeric",
    });
  });
  const humanData: number[] = Object.values(refactImpactDatesWeekly).map(
    (entry) => entry.human,
  );
  const refactData: number[] = Object.values(refactImpactDatesWeekly).map(
    (entry) => entry.refact,
  );

  const option = {
    tooltip: {
      trigger: "axis",
      axisPointer: {
        type: "shadow",
      },
    },
    legend: {},
    height: "200px",
    grid: {
      left: "3%",
      right: "4%",
      bottom: "3%",
      containLabel: true,
    },
    xAxis: [
      {
        type: "category",
        data: dates,
        axisLabel: {
          fontSize: 8,
        },
      },
    ],
    yAxis: [
      {
        type: "value",
      },
    ],
    series: [
      {
        name: "Human",
        type: "bar",
        stack: "Ad",
        data: humanData,
        barWidth: "80%",
      },
      {
        name: "Refact",
        type: "bar",
        stack: "Ad",
        data: refactData,
        barWidth: "80%",
      },
    ],
  };

  return (
    <Box mt="3" width="100%">
      <Text as="p" size="2">
        Refact vs Human
      </Text>
      <ReactEcharts
        option={option}
        style={{ width: "100%", height: "300px" }}
      />
      <Box>
        {dates.map((date: string, index: number) => (
          <Box key={index}>
            <Text size="1" as="p">
              Date: {date}
            </Text>
            <Text size="1" as="p">
              Human: {humanData[index]} ch.
            </Text>
            <Text size="1" mb="2" as="p">
              Refact: {refactData[index]} ch.
            </Text>
          </Box>
        ))}
      </Box>
    </Box>
  );
};
