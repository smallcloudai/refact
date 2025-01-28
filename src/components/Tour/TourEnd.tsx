import { Flex, Text } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";
import { useAppDispatch, useAppearance } from "../../hooks";
import { finish } from "../../features/Tour";

export const TourEnd = () => {
  const { appearance } = useAppearance();

  const dispatch = useAppDispatch();
  const onPressNext = () => {
    dispatch(finish());
  };

  return (
    <Flex
      direction="column"
      gap="2"
      maxWidth="540px"
      m="8px"
      style={{ alignSelf: "center" }}
    >
      <TourBox style={{ gap: "15px", alignSelf: "center" }}>
        <TourTitle title="Your Refact product tour is finished!" />
        <Text
          style={{
            color: appearance === "light" ? "white" : "black",
            whiteSpace: "pre-line",
          }}
        >
          {`There are more things in Refact:\n- our on-prem version\n- custom instructions`}
        </Text>
        <TourButton title="Ready to use" onClick={onPressNext} />
      </TourBox>
    </Flex>
  );
};
