import { Flex, Text, useThemeContext } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";

export type WelcomeProps = {
  onPressNext: () => void;
};

export const Welcome: React.FC<WelcomeProps> = ({
  onPressNext,
}: WelcomeProps) => {
  const appearance = useThemeContext().appearance;

  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <TourBox style={{ gap: "15px" }}>
        <TourTitle title="Welcome to Refact.ai!" />
        <Text
          style={{
            color: appearance == "dark" ? "black" : "white",
          }}
        >
          {"This is a product tour: helpful tips for you to start."}
        </Text>
        <TourButton title="Get Started" onClick={onPressNext} />
      </TourBox>
    </Flex>
  );
};
