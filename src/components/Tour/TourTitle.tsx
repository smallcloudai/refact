import { Flex, Text, useThemeContext } from "@radix-ui/themes";
import imgUrl from "../../../public/favicon.png";

export type TourTitle = {
  title: string;
};

export function TourTitle({ title }: TourTitle) {
  const appearance = useThemeContext().appearance;

  return (
    <Flex direction="row" style={{ alignItems: "flex-start" }}>
      <img
        src={imgUrl}
        width={28}
        height={28}
        style={{ marginTop: 5, marginBottom: 5 }}
      />
      <Text
        size="3"
        m="4"
        mt="0"
        mb="0"
        ml="2"
        style={{
          color: appearance == "dark" ? "black" : "white",
          // fontSize: 14,
          // margin: 4,
          // marginTop: 0,
          // marginLeft: 8,
          paddingRight: 30,
          alignSelf: "center",
        }}
      >
        {title}
      </Text>
    </Flex>
  );
}
