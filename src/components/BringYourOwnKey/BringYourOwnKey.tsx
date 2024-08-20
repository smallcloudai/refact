import { Button, Flex, Text } from "@radix-ui/themes";

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
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <Text size="3">Bring Your Own Key</Text>
      <Text size="2">
        Allows you to connect to any OpenAi or Huggingface style server.
      </Text>
      <Flex gap="2">
        <Button variant="outline" mr="auto" onClick={goBack}>
          {"< Back"}
        </Button>
        <Button ml="auto" type="submit" onClick={onSubmit}>
          {"Create custom BYOK file"}
        </Button>
      </Flex>
    </Flex>
  );
};
