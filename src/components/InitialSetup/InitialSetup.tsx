import { Button, Flex, Radio, RadioCards, Text } from "@radix-ui/themes";
import { useState } from "react";

export type Host =
  | "cloud"
  | "self-hosting"
  | "enterprise"
  | "bring-your-own-key";

export type InitialSetupProps = {
  onPressNext: (host: Host) => void;
};

export const InitialSetup: React.FC<InitialSetupProps> = ({
  onPressNext,
}: InitialSetupProps) => {
  const [selected, setSelected] = useState<Host | undefined>(undefined);

  const onValueChange = (value: string) => {
    setSelected(value as Host);
  };

  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <Text size="4">Refact plugin initial setup:</Text>
      <RadioCards.Root
        style={{ display: "flex", flexDirection: "column", gap: "6px" }}
        value={selected}
        onValueChange={onValueChange}
      >
        <RadioCards.Item
          value="cloud"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex gap="6px">
            <Radio value="cloud" checked={selected === "cloud"} />
            <Text size="3">Cloud</Text>
          </Flex>
          <Text size="2">- Easy to start</Text>
          <Text size="2">- Free tier</Text>
          <Text size="2">
            - You can opt-in for code snippets collection to help this open
            source project, off by default
          </Text>
        </RadioCards.Item>
        <RadioCards.Item
          value="self-hosting"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex gap="6px">
            <Radio value="self-hosting" checked={selected === "self-hosting"} />
            <Text size="3">Self-hosting</Text>
          </Flex>
          <Text size="2">- Uses your own server</Text>
          <Text size="2">- Your code never leaves your control</Text>
        </RadioCards.Item>
        <RadioCards.Item
          value="enterprise"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex gap="6px">
            <Radio value="enterprise" checked={selected === "enterprise"} />
            <Text size="3">Enterprise</Text>
          </Flex>
          <Text size="2">{"- Doesn't connect to a public cloud"}</Text>
          <Text size="2">- Uses your private server only</Text>
          <Text size="2">
            - Sends telemetry and code snippets to your private server
          </Text>
        </RadioCards.Item>
        <RadioCards.Item
          value="bring-your-own-key"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex gap="6px">
            <Radio
              value="bring-your-own-key"
              checked={selected === "bring-your-own-key"}
            />
            <Text size="3">Bring your own key</Text>
          </Flex>
          <Text size="2">
            - Connect to any OpenAI or HuggingFace style server.
          </Text>
        </RadioCards.Item>
      </RadioCards.Root>
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
        {"Next >"}
      </Button>
    </Flex>
  );
};
