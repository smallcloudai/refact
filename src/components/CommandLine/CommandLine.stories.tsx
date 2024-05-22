import type { Meta, StoryObj } from "@storybook/react";
import { CommandLine } from ".";

const meta = {
  title: "Components/Command Line",
  component: CommandLine,
  args: {
    command: "",
    error: false,
    args: {},
    result: "",
  },
} as Meta<typeof CommandLine>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const Cat: Story = {
  args: {
    command: "cat",
    error: false,
    args: {
      file: "test.txt",
    },
    result:
      "File tests/emergency_frog_situation/frog.py:1-29\n```python\nimport numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n```",
  },
};
