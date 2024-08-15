import { Flex, useThemeContext } from "@radix-ui/themes";
import { CSSProperties, ReactNode } from "react";

export type TourBubbleProps = {
  children?: ReactNode;
  style?: CSSProperties;
};

export function TourBox({ children, style }: TourBubbleProps) {
  const appearance = useThemeContext().appearance;

  return (
    <Flex
      direction="column"
      style={{
        position: "relative",
        backgroundColor: appearance === "dark" ? "white" : "black",
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
