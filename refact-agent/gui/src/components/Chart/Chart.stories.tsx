import type { Meta } from "@storybook/react";
import { Chart } from "./Chart";
import { Box } from "@radix-ui/themes";

const meta = {
  title: "Chart",
  component: Chart,
} satisfies Meta<typeof Chart>;

export default meta;

export const Primary = () => {
  return (
    <Box
      p="2"
      style={{
        width: "260px",
        backgroundColor: "color(display-p3 0.004 0.004 0.204 / 0.059)",
        height: "100%",
      }}
    >
      <Chart
        refactImpactDatesWeekly={{
          "2023-12-15": {
            completions: 14,
            human: 52,
            langs: [".py", ".rs"],
            refact: 203,
            refact_impact: 0.7960784435272217,
            total: 255,
          },
          "2023-12-22": {
            completions: 219,
            human: 2063,
            langs: [".py", ".cpp"],
            refact: 6149,
            refact_impact: 0.7487822771072388,
            total: 8212,
          },
          "2023-12-27": {
            completions: 15,
            human: 30,
            langs: [".py"],
            refact: 480,
            refact_impact: 0.9411764740943909,
            total: 510,
          },
          "2024-01-04": {
            completions: 12,
            human: 1772,
            langs: [".rs"],
            refact: 303,
            refact_impact: 0.14602409303188324,
            total: 2075,
          },
          "2024-01-09": {
            completions: 4,
            human: 33,
            langs: [".py"],
            refact: 166,
            refact_impact: 0.8341708779335022,
            total: 199,
          },
          "2024-01-24": {
            completions: 107,
            human: 10732,
            langs: [".rs"],
            refact: 3739,
            refact_impact: 0.2583788335323334,
            total: 14471,
          },
          "2024-02-02": {
            completions: 149,
            human: 22550,
            langs: [".rs", ".py", ".txt"],
            refact: 5851,
            refact_impact: 0.20601387321949005,
            total: 28401,
          },
          "2024-02-05": {
            completions: 5,
            human: 1976,
            langs: [".rs", ".txt"],
            refact: 233,
            refact_impact: 0.10547759383916855,
            total: 2209,
          },
        }}
      />
    </Box>
  );
};
