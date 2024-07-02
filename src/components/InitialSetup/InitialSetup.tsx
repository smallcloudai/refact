import { Button, Card, Flex, RadioGroup, Text } from "@radix-ui/themes";
import { useState } from "react";

type Host = "cloud" | "self-hosting" | "enterprise";

export type InitialSetupProps = {
  onPressNext: (host: Host) => void;
};

export const InitialSetup: React.FC<InitialSetupProps> = ({
  onPressNext,
}: InitialSetupProps) => {
  const [selected, setSelected] = useState<Host | undefined>(undefined);

  return (
    <Flex direction="column" gap="2">
      <Text size="4">Refact plugin initial setup:</Text>
      <RadioGroup.Root
        style={{ gap: 10 }}
        value={selected}
        onValueChange={(value) => {
          setSelected(value as Host);
        }}
      >
        <Card style={{ display: "flex", flexDirection: "column" }}>
          <RadioGroup.Item value="cloud">
            <Text size="3">Cloud</Text>
          </RadioGroup.Item>
          <Text size="2">- Easy to start</Text>
          <Text size="2">- Free tier</Text>
          <Text size="2">
            - You can opt-in for code snippets collection to help this open
            source project, off by default
          </Text>
        </Card>
        <Card style={{ display: "flex", flexDirection: "column" }}>
          <RadioGroup.Item value="self-hosting">
            <Text size="3">Self-hosting</Text>
          </RadioGroup.Item>
          <Text size="2">- Uses your own server</Text>
          <Text size="2">- Your code never leaves your control</Text>
        </Card>
        <Card style={{ display: "flex", flexDirection: "column" }}>
          <RadioGroup.Item value="enterprise">
            <Text size="3">Enterprise</Text>
          </RadioGroup.Item>
          <Text size="2">{"- Doesn't connect to a public cloud"}</Text>
          <Text size="2">- Uses your private server only</Text>
          <Text size="2">
            - Sends telemetry and code snippets to your private server
          </Text>
        </Card>
      </RadioGroup.Root>
      <Button
        variant="outline"
        ml="auto"
        disabled={selected === undefined}
        onClick={() => {
          if (selected) {
            onPressNext(selected);
          }
        }}
      >
        Next
      </Button>
    </Flex>
  );
};
