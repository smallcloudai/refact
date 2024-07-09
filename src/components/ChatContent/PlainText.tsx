import React from "react";
import { Text, Container } from "@radix-ui/themes";

export type PlainTextProps = {
  children: string;
};

export const PlainText: React.FC<PlainTextProps> = ({ children }) => {
  return (
    <Container>
      <Text>{children}</Text>
    </Container>
  );
};
