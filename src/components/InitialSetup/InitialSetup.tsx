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
      <Text size="4">Initial Setup</Text>
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
            <Text size="3">Refact Cloud</Text>
          </Flex>
          <Text size="2">- Easy to start</Text>
          <Text size="2">- Free tier</Text>
          <Text size="2">- PRO plan with a great choice of models</Text>
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
          <Text size="2">- Uses your private server only</Text>
          <Text size="2">- Sends telemetry to your private server</Text>
          <Text size="2">- Fine-tune completion models to your codebase</Text>
          <Text size="2">- Customize for the entire team</Text>
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
            <Text size="3">Self-Hosting with Refact Server</Text>
          </Flex>
          {/* TODO: add link to self-hosting doc */}
          <Text size="2">- Uses your own server</Text>
          <Text size="2">- Fine-tune completion models to your codebase</Text>
          <Text size="2">- Your code never leaves your control</Text>
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
            <Text size="3">Bring Your Own Key</Text>
          </Flex>
          <Text size="2">
            - Connect to any OpenAI- or HuggingFace-style server
          </Text>
          <Text size="2">
            - Separate endpoints and keys for chat, completion, embedding
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
