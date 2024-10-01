import { Button, Flex, Text } from "@radix-ui/themes";
import { ChevronLeftIcon } from "@radix-ui/react-icons";
import { Link } from "../Link";

export interface BringYourOwnKeyProps {
  goBack: () => void;
  next: () => void;
}

export const BringYourOwnKey: React.FC<BringYourOwnKeyProps> = ({
  goBack,
  next,
}: BringYourOwnKeyProps) => {
  const onSubmit = () => {
    next();
  };

  return (
    <Flex
      direction="column"
      gap="2"
      maxWidth="540px"
      m="8px"
      style={{ alignSelf: "center" }}
    >
      <Text size="4">Bring Your Own Key</Text>
      <Text size="2">
        Allows you to connect to any service that has OpenAI- or
        HuggingFace-style API.
      </Text>
      <Text size="2">
        Works with llama.cpp, OpenRouter, or almost any other service. You can
        set up separate endpoints and keys for chat, completion, embedding.
      </Text>
      <Text size="2">
        Please report any problems to the{" "}
        <Link href="https://github.com/smallcloudai/refact-lsp/issues">
          refact-lsp issues
        </Link>{" "}
        page. Also, report positive experience to{" "}
        <Link href="https://www.smallcloud.ai/discord">discord</Link>!
      </Text>
      <Text size="2">
        The button below opens bring-your-own-key.yaml in the IDE.
      </Text>
      <Flex gap="2" mt="1">
        <Button
          color="gray"
          highContrast
          variant="outline"
          mr="auto"
          onClick={goBack}
        >
          <ChevronLeftIcon />
          {"Back"}
        </Button>
        <Button
          color="gray"
          highContrast
          variant="solid"
          ml="auto"
          type="submit"
          onClick={onSubmit}
        >
          {"Edit BYOK file"}
        </Button>
      </Flex>
    </Flex>
  );
};
