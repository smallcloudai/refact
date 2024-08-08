import React from "react";
import type { ContextMemory } from "../../services/refact";
import { Badge, Flex, Text } from "@radix-ui/themes";
import { CardStackIcon } from "@radix-ui/react-icons";
import { HoverCard, Container } from "@radix-ui/themes";

const Note: React.FC<ContextMemory> = ({ memo_id, memo_text }) => {
  return (
    <HoverCard.Root>
      <HoverCard.Trigger>
        <Badge>
          <CardStackIcon />
        </Badge>
      </HoverCard.Trigger>
      <HoverCard.Content>
        <Flex gap="3" direction="column">
          <Text size="1" as="div" weight="light" wrap="wrap" trim="both">
            {memo_id}
          </Text>
          <Text size="2" as="div" wrap="wrap">
            {memo_text}
          </Text>
        </Flex>
      </HoverCard.Content>
    </HoverCard.Root>
  );
};

export const MemoryContent: React.FC<{ items: ContextMemory[] }> = ({
  items,
}) => {
  return (
    <Container>
      <Flex gap="2" p="1" wrap="wrap">
        {items.map((item, index) => {
          const key = `${item.memo_id}-${index}`;
          return <Note key={key} {...item} />;
        })}
      </Flex>
    </Container>
  );
};
