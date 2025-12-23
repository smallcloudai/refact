import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ComboBox, type ComboBoxProps } from "./ComboBox";
import { TextArea } from "../TextArea";
import { Card } from "@radix-ui/themes";
import { useDebounceCallback } from "usehooks-ts";

async function getCommands(query: string, cursor: number) {
  return fetch("/v1/at-command-completion", {
    method: "POST",
    body: JSON.stringify({ query, cursor, top_n: 5 }),
  })
    .then((res) => res.json())
    .then((json) => json as ComboBoxProps["commands"])
    .catch((err) => {
      // eslint-disable-next-line no-console
      console.error(err);
    });
}

const App: React.FC<ComboBoxProps> = (props) => {
  const [value, setValue] = React.useState<string>("");
  const [commands, setCommands] = React.useState<ComboBoxProps["commands"]>({
    completions: [],
    replace: [0, 0],
    is_cmd_executable: false,
  });

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const handleCommandCompletion = React.useCallback(
    useDebounceCallback((query: string, cursor: number) => {
      void getCommands(query, cursor).then((res) => res && setCommands(res));
    }, 0),
    [],
  );
  return (
    <Card size="5" m="8">
      <ComboBox
        {...props}
        commands={commands}
        value={value}
        onChange={setValue}
        requestCommandsCompletion={handleCommandCompletion}
      />
    </Card>
  );
};

const meta = {
  title: "ComboBox V2",
  component: App,
} satisfies Meta<typeof ComboBox>;

export default meta;

export const Default: StoryObj<typeof ComboBox> = {
  args: {
    onSubmit: () => ({}),
    placeholder: "Type @ for commands",
    render: (props) => <TextArea {...props} />,
  },
};
