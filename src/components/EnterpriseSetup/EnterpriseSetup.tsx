import { Button, Flex, Text, TextField } from "@radix-ui/themes";

export const EnterpriseSetup: React.FC = () => {
  return (
    <Flex direction="column" gap="2">
      <Text size="2">
        You should have corporate endpoint URL and personal API key. Please
        contact your system administrator.
      </Text>
      <Text size="2">Endpoint Address</Text>
      <TextField.Root />
      <Text size="2">API Key</Text>
      <TextField.Root />
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
