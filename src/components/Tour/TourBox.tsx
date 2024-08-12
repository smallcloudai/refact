import { Flex } from "@radix-ui/themes";
import { CSSProperties, ReactNode } from "react";

export type TourBubbleProps = {
  children?: ReactNode;
  style?: CSSProperties;
};

export function TourBox({ children, style }: TourBubbleProps) {
  return (
    <Flex
      direction="column"
      style={{
        position: "relative",
        backgroundColor: "white",
        borderRadius: "5px",
        minHeight: "60px",
        justifyContent: "center",
        padding: "10px",
        alignSelf: "stretch",
        ...style,
      }}
    >
      {children}
    </Flex>
  );
}
