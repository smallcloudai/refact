import { Button, Flex, Radio, RadioCards, Text } from "@radix-ui/themes";
import { ChevronLeftIcon } from "@radix-ui/react-icons";
import { useEffect, useRef, useState } from "react";
import { useLogin } from "../../hooks";
import { isGoodResponse } from "../../services/smallcloud";

export interface CloudLoginProps {
  goBack: () => void;
}

// TODO: duplicated else where, could be a component
const bulletStyle = {
  marginRight: "5px",
  verticalAlign: "middle",
  display: "inline-flex",
  width: "4px",
  height: "4px",
  backgroundColor: "var(--gray-12)",
  borderRadius: "50%",
};

export const CloudLogin: React.FC<CloudLoginProps> = ({
  goBack,
}: CloudLoginProps) => {
  const [selected, setSelected] = useState<"free" | "pro">("pro");
  const loginButton = useRef<HTMLButtonElement>(null);

  const { loginThroughWeb, cancelLogin, loginWithKey, polling } = useLogin();

  useEffect(() => {
    cancelLogin.current();
  }, [cancelLogin, selected]);

  useEffect(() => {
    const { current } = loginButton;
    if (current === null) {
      return;
    }

    if (polling.isLoading) {
      const loadingText = "Fetching API Key ";
      const animationFrames = ["/", "|", "\\", "-"];
      let index = 0;

      const interval = setInterval(() => {
        current.innerText = `${loadingText} ${animationFrames[index]}`;
        index = (index + 1) % animationFrames.length;
      }, 100);

      return () => {
        clearInterval(interval);
      };
    } else {
      current.innerText = "Log In";
    }
  }, [loginButton, polling.isLoading]);

  useEffect(() => {
    if (isGoodResponse(polling.data)) {
      const apiKey = polling.data.secret_key;
      loginWithKey(apiKey);
    }
  }, [polling.data, loginWithKey]);

  const onValueChange = (value: string) => {
    setSelected(value as "free" | "pro");
  };

  const onLoginOrCreateAccount = ({
    plan,
    isLogin,
  }: {
    plan?: "free" | "pro";
    isLogin?: boolean;
  }): void => {
    if (!isLogin && plan) {
      setSelected(plan);
      loginThroughWeb(plan === "pro");
      return;
    }
    loginThroughWeb(false);
  };

  return (
    <Flex
      direction="column"
      gap="2"
      maxWidth="540px"
      m="8px"
      style={{ alignSelf: "center" }}
    >
      <Text size="3">Already have a Refact.ai account?</Text>
      <Button
        ref={loginButton}
        onClick={() =>
          onLoginOrCreateAccount({
            isLogin: true,
          })
        }
        color="gray"
        highContrast
        variant="solid"
        style={{
          width: "100%",
          fontFamily: polling.isLoading ? "monospace" : undefined,
        }}
        disabled={polling.isLoading}
      >
        Log In
      </Button>
      <Text size="3" mt="4">
        New to Refact.ai? Choose a plan.
      </Text>

      <RadioCards.Root
        color="tomato"
        style={{ display: "flex", flexDirection: "column", gap: "6px" }}
        value={selected}
        onValueChange={onValueChange}
      >
        <RadioCards.Item
          value="free"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex align="center">
            <Radio mr="2" value="free" checked={selected === "free"} />
            <Text size="3">Free plan</Text>
          </Flex>
          <Flex pl="5" direction="column">
            <Text size="2">
              <i style={bulletStyle}></i>Code completions: Refact 1.6 model
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>In-IDE Chat: GPT-4o mini
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Toolbox (refactor code, find bugs,
              etc.)
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>2048-context length for completions
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>8k context length for chat
            </Text>
          </Flex>
        </RadioCards.Item>
        <RadioCards.Item
          value="pro"
          style={{
            flexDirection: "column",
            alignItems: "flex-start",
            gap: 0,
          }}
        >
          <Flex align="center">
            <Radio mr="2" value="pro" checked={selected === "pro"} />
            <Text size="3">Pro plan</Text>
            <Flex
              pl="5px"
              pr="5px"
              pt="3px"
              pb="3px"
              style={{
                position: "absolute",
                right: "var(--space-3)",
                backgroundColor: "#E7150D",
                color: "white",
                borderRadius: "4px",
              }}
            >
              <Text size="1" as="div" weight="bold">
                1 MONTH FREE
              </Text>
            </Flex>
          </Flex>
          <Flex pl="5" direction="column">
            <Text size="2" mt="1">
              As in the Free plan, plus:
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>Code completions: StarCode2-3B model
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>In-IDE Chat: GPT-4o, GPT-4 turbo,
              Claude 3.5 Sonnet
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>More AI models for Toolbox
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>x2 context length for completions
            </Text>
            <Text size="2">
              <i style={bulletStyle}></i>x4 context length for chat
            </Text>
          </Flex>
        </RadioCards.Item>
      </RadioCards.Root>
      <Flex gap="2">
        <Button
          mr="auto"
          onClick={goBack}
          color="gray"
          highContrast
          variant="outline"
        >
          <ChevronLeftIcon />
          {"Back"}
        </Button>
        <Button
          ml="auto"
          onClick={() => onLoginOrCreateAccount({ plan: selected })}
          color="gray"
          highContrast
          variant="solid"
        >
          {"Create Account"}
        </Button>
      </Flex>
    </Flex>
  );
};
