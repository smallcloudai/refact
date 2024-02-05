import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ComboBox, type ComboBoxProps } from "./ComboBox";
import { TextArea } from "../TextArea";
import { Card } from "@radix-ui/themes";

const App: React.FC<ComboBoxProps> = (props) => {
  const [value, setValue] = React.useState<string>("");
  const [selectedCommand, setSelectedCommand] = React.useState<string>("");
  return (
    <Card size="5" m="8">
      <ComboBox
        {...props}
        value={value}
        onChange={setValue}
        selectedCommand={selectedCommand}
        setSelectedCommand={setSelectedCommand}
        executeCommand={() => console.log("execute command")}
      />
    </Card>
  );
};

const meta = {
  title: "ComboBox",
  component: App,
} satisfies Meta<typeof ComboBox>;

export default meta;

export const Default: StoryObj<typeof ComboBox> = {
  args: {
    commands: ["@file"],
    requestCommandsCompletion: () => ({}),
    commandArguments: ["/foo", "/bar"],
    // value: value,
    // onChange: () => ({}),
    onSubmit: () => ({}),
    placeholder: "Type @ for commands",
    render: (props) => <TextArea {...props} />,
  },
};
