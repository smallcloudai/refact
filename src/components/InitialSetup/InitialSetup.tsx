import { Button, Flex, Radio, RadioCards, Text } from "@radix-ui/themes";
import { ChevronRightIcon } from "@radix-ui/react-icons";
import { useState } from "react";

export type Host =
  | "cloud"
  | "self-hosting"
  | "enterprise"
  | "bring-your-own-key";

export type InitialSetupProps = {
  onPressNext: (host: Host) => void;
};

// TODO: duplicated code from another setup, this could be a component
const bulletStyle = {
  marginRight: "5px",
  verticalAlign: "middle",
  display: "inline-flex",
  width: "4px",
  height: "4px",
  backgroundColor: "var(--gray-12)",
  borderRadius: "50%",
};

export const InitialSetup: React.FC<InitialSetupProps> = ({
  onPressNext,
}: InitialSetupProps) => {
  const [selected, setSelected] = useState<Host | undefined>(undefined);

  const onValueChange = (value: string) => {
    setSelected(value as Host);
  };

  return (
    <Flex
      direction="column"
      gap="2"
      maxWidth="540px"
      m="8px"
      style={{ alignSelf: "center" }}
    >
      <Text size="4">Which Refact.ai setup would you like to use?</Text>
      <RadioCards.Root
        color="tomato"
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
          <Flex align="center" mb="1">
            <Radio mr="2" value="cloud" checked={selected === "cloud"} />
            <Text weight="medium" size="3">
              Refact Cloud
            </Text>
          </Flex>
          <Flex pl="5" direction="column">
            <Text size="2">
              <i style={bulletStyle}></i>Easy to start
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Free tier
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>PRO plan with a great choice of models
            </Text>
          </Flex>
        </RadioCards.Item>
        <RadioCards.Item
          value="enterprise"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex align="center" mb="1">
            <Radio
              mr="2"
              value="enterprise"
              checked={selected === "enterprise"}
            />
            <Text weight="medium" size="3">
              Enterprise
            </Text>
          </Flex>
          <Flex pl="5" direction="column">
            <Text size="2">
              <i style={bulletStyle}></i>Uses your private server only
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Sends telemetry to your private server
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Fine-tune completion models to your
              codebase
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Customize for the entire team
            </Text>
          </Flex>
        </RadioCards.Item>
        <RadioCards.Item
          value="self-hosting"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex align="center" mb="1">
            <Radio
              mr="2"
              value="self-hosting"
              checked={selected === "self-hosting"}
            />
            <Text weight="medium" size="3">
              Self-Hosting with Refact Server
            </Text>
          </Flex>
          {/* TODO: add link to self-hosting doc */}
          <Flex pl="5" direction="column">
            <Text size="2">
              <i style={bulletStyle}></i>Uses your own server
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Fine-tune completion models to your
              codebase
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Your code never leaves your control
            </Text>
          </Flex>
        </RadioCards.Item>
        <RadioCards.Item
          value="bring-your-own-key"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex align="center" mb="1">
            <Radio
              mr="2"
              value="bring-your-own-key"
              checked={selected === "bring-your-own-key"}
            />
            <Text weight="medium" size="3">
              Bring Your Own Key
            </Text>
          </Flex>
          <Flex pl="5" direction="column">
            <Text size="2">
              <i style={bulletStyle}></i>Connect to any OpenAI- or
              HuggingFace-style server
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Separate endpoints and keys for chat,
              completion, embedding
            </Text>
          </Flex>
        </RadioCards.Item>
      </RadioCards.Root>
      <Button
        color="gray"
        highContrast
        variant="solid"
        ml="auto"
        disabled={selected === undefined}
        onClick={() => {
          if (selected) {
            onPressNext(selected);
          }
        }}
      >
        {"Next"}
        <ChevronRightIcon />
      </Button>
    </Flex>
  );
};
