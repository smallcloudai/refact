import React from "react";
import { Box, Text } from "@radix-ui/themes";
import { RefactTableImpactDateObj } from "../../services/refact";
import ReactEChartsCore from "echarts-for-react/lib/core";
import * as echarts from "echarts/core";
import { BarChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  AxisPointerComponent,
  TitleComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";

import { Spinner } from "../Spinner";
echarts.use([
  TitleComponent,
  TooltipComponent,
  GridComponent,
  BarChart,
  CanvasRenderer,
  AxisPointerComponent,
]);

export const Chart: React.FC<{
  refactImpactDatesWeekly: Record<string, RefactTableImpactDateObj> | null;
}> = ({ refactImpactDatesWeekly }) => {
  if (refactImpactDatesWeekly === null) {
    return <Spinner />;
  }

  const dates: string[] = Object.keys(refactImpactDatesWeekly).map((date) => {
    return new Date(date).toLocaleString(undefined, {
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
      <ReactEChartsCore
        echarts={echarts}
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
