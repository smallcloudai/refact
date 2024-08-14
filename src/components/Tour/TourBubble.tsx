import { Flex, Link } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import { RootState } from "../../app/store";
import { close, next } from "../../features/Tour";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { useEffect, useState } from "react";

export type TourBubbleProps = {
  text: string;
  step: number;
  down: boolean;
  isPointing?: boolean;
  target: HTMLElement | null;
  containerWidth?: string;
  onPage: string;
  page: string;
  onNext?: () => void;
};

export function TourBubble({
  text,
  step,
  target,
  down,
  containerWidth,
  onPage,
  page,
  isPointing,
  onNext,
}: TourBubbleProps) {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const { width: windowWidth, height: windowHeight } = useWindowDimensions();
  const [pos, setPos] = useState<DOMRect | undefined>(undefined);

  const isBubbleOpen = state.type === "in_progress" && state.step === step;

  if (isPointing === undefined) {
    isPointing = true;
  }

  // TODO: find a better way of doing this
  // This code is there to force a rerender if target is null
  useEffect(() => {
    if (target === null || page !== onPage) {
      setPos(undefined);
    } else {
      const newPos = target.getBoundingClientRect();
      if (
        pos?.left !== newPos.left ||
        pos.right !== newPos.right ||
        pos.top !== newPos.top ||
        pos.bottom !== newPos.bottom
      ) {
        setPos(newPos);
      }
    }
  }, [page, onPage, target, pos, setPos, windowWidth, windowHeight]);

  if (pos === undefined) {
    return <></>;
  }

  const centX = (pos.left + pos.right) / 2 - windowWidth / 2;

  return (
    isBubbleOpen && (
      <Flex
        style={{
          flexDirection: "column",
          position: "fixed",
          height: 0,
          width: "100%",
          alignSelf: "center",
          top: down ? pos.bottom : pos.top,
          zIndex: 100,
        }}
      >
        <Flex
          style={{
            position: "absolute",
            width: containerWidth ?? "min(calc(100% - 20px), 540px)",
            flexDirection: "column",
            alignSelf: "center",
            bottom: down ? "auto" : 0,
            top: down ? 0 : "auto",
          }}
        >
          {down && isPointing && (
            <Flex
              style={{
                width: 0,
                height: 0,
                borderLeft: "15px solid transparent",
                borderRight: "15px solid transparent",
                borderBottom: "15px solid white",
                alignSelf: "center",
                position: "relative",
                left: centX,
              }}
            />
          )}
          <TourBox>
            <TourTitle title={text} />
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "8px",
                top: "1px",
                color: "black",
              }}
              onClick={() => {
                dispatch(close());
              }}
            >
              x
            </Link>
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "10px",
                bottom: "10px",
                color: "#3312a3",
              }}
              onClick={() => {
                dispatch(next());
                if (onNext) onNext();
              }}
            >
              next
            </Link>
          </TourBox>
          {down || !isPointing || (
            <Flex
              style={{
                width: 0,
                height: 0,
                borderLeft: "15px solid transparent",
                borderRight: "15px solid transparent",
                borderTop: "15px solid white",
                alignSelf: "center",
                position: "relative",
                left: centX,
              }}
            />
          )}
        </Flex>
      </Flex>
    )
  );
}
