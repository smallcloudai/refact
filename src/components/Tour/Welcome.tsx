import { Flex, Text } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";

export type WelcomeProps = {
  onPressNext: () => void;
};

export const Welcome: React.FC<WelcomeProps> = ({
  onPressNext,
}: WelcomeProps) => {
  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <TourBox style={{ gap: "15px" }}>
        <TourTitle title="Welcome to Refact.ai!" />
        <Text style={{ color: "black" }}>
          {"You're using the most customizable AI Coding Assistant."}
        </Text>
        <TourButton title="Get Started" onClick={onPressNext} />
      </TourBox>
    </Flex>
  );
};
