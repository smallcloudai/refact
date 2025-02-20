import { Card, Flex, Text } from "@radix-ui/themes";
import styles from "./UsageCounter.module.css";
import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";

export const UsageCounter = () => {
  return (
    <Card className={styles.usageCounterContainer}>
      <Flex align="center">
        <ArrowUpIcon width="12" height="12" />
        <Text size="1">1.2k</Text>
      </Flex>
      <Flex align="center">
        <ArrowDownIcon width="12" height="12" />
        <Text size="1">12k</Text>
      </Flex>
    </Card>
  );
};
