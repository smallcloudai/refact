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
import { useAppearance } from "../../hooks";

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
  const { isDarkMode } = useAppearance();

  if (refactImpactDatesWeekly === null) {
    return <Spinner spinning />;
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
      top: "10%",
      containLabel: true,
    },
    xAxis: [
      {
        type: "category",
        data: dates,
        axisLine: {
          lineStyle: {
            color: isDarkMode ? "#ffffff" : "#646464",
          },
        },
      },
    ],
    yAxis: [
      {
        type: "value",
        name: "char.",
        nameTextStyle: {
          align: "right",
        },
        axisLine: {
          lineStyle: {
            color: isDarkMode ? "#ffffff" : "#646464",
          },
        },
      },
    ],
    series: [
      {
        name: "Human",
        type: "bar",
        stack: "Ad",
        data: humanData,
        barWidth: "80%",
        itemStyle: { normal: { color: "#91cc75" } },
      },
      {
        name: "Refact",
        type: "bar",
        stack: "Ad",
        data: refactData,
        barWidth: "80%",
        itemStyle: { normal: { color: "#5470c6" } },
      },
    ],
  };

  return (
    <Box mt="3" width="100%">
      <Text as="p" size="2" mt="5">
        Refact vs Human
      </Text>
      <ReactEChartsCore
        echarts={echarts}
        option={option}
        style={{ width: "100%", height: "300px" }}
      />
    </Box>
  );
};
