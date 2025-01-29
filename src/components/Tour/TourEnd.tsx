import { Flex, Text } from "@radix-ui/themes";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { TourButton } from "./TourButton";
import { useAppDispatch, useAppearance, useOpenUrl } from "../../hooks";
import { finish } from "../../features/Tour";
import { Link } from "../Link";

export const TourEnd = () => {
  const { appearance } = useAppearance();
  const openUrl = useOpenUrl();
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
      <TourBox
        style={{
          gap: "15px",
          alignSelf: "center",
          color: appearance === "light" ? "white" : "black",
          whiteSpace: "pre-line",
        }}
      >
        <TourTitle title="Your Refact product tour is finished!" />
        <Flex direction="column">
          <Text mb="1">There are more things to explore in Refact!</Text>
          <Text>
            -{" "}
            <Link
              style={{ color: "black", textDecoration: "underline" }}
              onClick={() => openUrl("https://docs.refact.ai")}
            >
              Check out our documentation
            </Link>
          </Text>
        </Flex>
        <TourButton title="Ready to use" onClick={onPressNext} />
      </TourBox>
    </Flex>
  );
};
