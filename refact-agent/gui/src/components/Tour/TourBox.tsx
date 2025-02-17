import { Flex } from "@radix-ui/themes";
import { CSSProperties, ReactNode } from "react";
import { useAppearance } from "../../hooks";

export type TourBubbleProps = {
  children?: ReactNode;
  style?: CSSProperties;
};

export function TourBox({ children, style }: TourBubbleProps) {
  const { appearance } = useAppearance();
  const backgroundColorForTourBox = appearance === "light" ? "black" : "white";

  return (
    <Flex
      direction="column"
      style={{
        position: "relative",
        backgroundColor: backgroundColorForTourBox,
        borderRadius: "5px",
        minHeight: "60px",
        //TODO: justify prop
        justifyContent: "center",
        // TODO: padding prop
        padding: "10px",
        alignSelf: "stretch",
        maxWidth: 550,
        ...style,
      }}
    >
      {children}
    </Flex>
  );
}
