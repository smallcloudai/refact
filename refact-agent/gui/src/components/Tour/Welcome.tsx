import { Flex, Text } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";
import { useAppearance } from "../../hooks";

export type WelcomeProps = {
  onPressNext: () => void;
};

export const Welcome: React.FC<WelcomeProps> = ({
  onPressNext,
}: WelcomeProps) => {
  const { appearance } = useAppearance();

  return (
    <Flex
      direction="column"
      gap="2"
      maxWidth="540px"
      m="8px"
      style={{ alignSelf: "center" }}
    >
      <TourBox style={{ gap: "15px" }}>
        <TourTitle title="Welcome to Refact.ai!" />
        <Text
          style={{
            color: appearance == "light" ? "white" : "black",
          }}
        >
          {"This is a product tour: helpful tips for you to start."}
        </Text>
        <TourButton title="Get Started" onClick={onPressNext} />
      </TourBox>
    </Flex>
  );
};
