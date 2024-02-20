import type { Meta } from "@storybook/react";
import { Table } from "./Table";
import { Box } from "@radix-ui/themes";

const meta = {
  title: "Table",
  component: Table,
} satisfies Meta<typeof Table>;

export default meta;

export const Primary = () => {
  return (
    <Box
      p="2"
      style={{
        width: "260px",
        backgroundColor: "color(display-p3 0.004 0.004 0.204 / 0.059)",
        height: "100vh",
      }}
    >
      <Table
        refactImpactTable={[
          {
            completions: 276,
            human: 31996,
            lang: ".rs",
            refact: 10092,
            refact_impact: 0.23978331685066223,
            total: 42088,
          },
          {
            completions: 243,
            human: 7110,
            lang: ".py",
            refact: 6929,
            refact_impact: 0.49355366826057434,
            total: 14039,
          },
          {
            completions: 6,
            human: 4,
            lang: ".cpp",
            refact: 103,
            refact_impact: 0.9626168012619019,
            total: 107,
          },
          {
            completions: 0,
            human: 98,
            lang: ".txt",
            refact: 0,
            refact_impact: 0.0,
            total: 98,
          },
        ]}
      />
    </Box>
  );
};
