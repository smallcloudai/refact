import { Button, Flex, Text, TextField } from "@radix-ui/themes";
import { Checkbox } from "../Checkbox";

export const CloudLogin: React.FC = () => {
  return (
    <Flex direction="column" gap="2">
      <Text weight="bold" size="4">
        Cloud inference
      </Text>
      <Text size="2">Quick login via website:</Text>
      <Button variant="outline">Login / Create Account</Button>
      <Text size="2" mt="2">
        Alternatively, paste an existing Refact API key here:
      </Text>
      <TextField.Root />
      <Text size="2" mt="4">
        Help Refact collect a dataset of corrected code completions! This will
        help to improve code suggestions more to your preferences, and it also
        will improve code suggestions for everyone else. Hey, we&#39;re not an
        evil corporation!
      </Text>
      <Checkbox>Send corrected code snippets.</Checkbox>
      <Text size="2">
        Basic telemetry is always on when using cloud inference, but it only
        sends errors and counters.{" "}
        <a href="https://github.com/smallcloudai/refact-lsp/blob/main/README.md#telemetry">
          How telemetry works in open source refact-lsp
        </a>
      </Text>
      <Flex gap="2">
        <Button variant="outline" mr="auto">
          Back
        </Button>
        <Button variant="outline" ml="auto">
          Next
        </Button>
      </Flex>
    </Flex>
  );
};
