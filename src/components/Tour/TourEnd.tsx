import { Flex, Text } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";
import { finish } from "../../features/Tour";

export type TourEndProps = {
  step: number;
};

export const TourEnd = ({ step }: TourEndProps) => {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const onPressNext = () => {
    dispatch(finish());
  };

  const isBubbleOpen = state.type === "in_progress" && state.step === step;

  return (
    isBubbleOpen && (
      <Flex
        direction="column"
        gap="2"
        m="8px"
        style={{
          position: "fixed",
          width: "100vw",
          height: "100%",
          backgroundColor: "rgba(0, 0, 0, 0.5)",
        }}
      >
        <TourBox
          style={{ gap: "15px", maxWidth: "540px", alignSelf: "center" }}
        >
          <TourTitle title="Your Refact.ai tour is over." />
          <Text style={{ color: "black" }}>
            {
              "You're now fully empowered to take advantage of all Refact's features!"
            }
          </Text>
          <TourButton title="Ready to use" onClick={onPressNext} />
        </TourBox>
      </Flex>
    )
  );
};
