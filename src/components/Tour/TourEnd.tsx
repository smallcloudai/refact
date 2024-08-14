import { Flex, Text } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";
import { useAppDispatch } from "../../app/hooks";
import { finish } from "../../features/Tour";

export const TourEnd = () => {
  const dispatch = useAppDispatch();
  const onPressNext = () => {
    dispatch(finish());
  };

  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <TourBox style={{ gap: "15px", alignSelf: "center" }}>
        <TourTitle title="Your Refact.ai tour is over." />
        <Text style={{ color: "black" }}>
          {
            "You're now fully empowered to take advantage of all Refact's features!"
          }
        </Text>
        <TourButton title="Ready to use" onClick={onPressNext} />
      </TourBox>
    </Flex>
  );
};
