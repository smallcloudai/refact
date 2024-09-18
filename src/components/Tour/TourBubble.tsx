import { Flex, Link } from "@radix-ui/themes";
import { useAppSelector, useAppDispatch, useAppearance } from "../../hooks";
import { RootState } from "../../app/store";
import { close, next } from "../../features/Tour";
import { useWindowDimensions } from "../../hooks/useWindowDimensions";
import { TourBox } from "./TourBox";
import { TourTitle } from "./TourTitle";
import { CSSProperties, ReactNode, useEffect, useState } from "react";

export type TourBubbleProps = {
  text: string;
  step: number;
  down: boolean;
  isPointing?: boolean;
  target: HTMLElement | null;
  containerWidth?: string;
  onPage: string;
  page: string;
  deltaY?: number;
  children?: ReactNode;
  onNext?: () => void;
  bubbleContainerStyles?: CSSProperties;
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
  deltaY,
  children,
  onNext,
  bubbleContainerStyles,
}: TourBubbleProps) {
  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.tour);
  const { width: windowWidth, height: windowHeight } = useWindowDimensions();
  const [pos, setPos] = useState<DOMRect | undefined>(undefined);
  const { appearance } = useAppearance();

  const isBubbleOpen = state.type === "in_progress" && state.step === step;

  if (isPointing === undefined) {
    isPointing = true;
  }

  // TODO: find a better way of doing this
  // This code is there to force a rerender if target is null
  useEffect(() => {
    const update = () => {
      if (target === null || page !== onPage) {
        if (pos !== undefined) {
          setPos(undefined);
        }
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
    };
    update();

    if (target !== null && page === onPage && isBubbleOpen) {
      const interval = setInterval(update, 100);
      return () => {
        clearInterval(interval);
      };
    }
  }, [
    page,
    onPage,
    target,
    pos,
    setPos,
    windowWidth,
    windowHeight,
    isBubbleOpen,
  ]);

  if (pos === undefined) {
    return <></>;
  }

  const centX = (pos.left + pos.right) / 2 - windowWidth / 2;
  const arrowColor = appearance == "light" ? "black" : "white";

  return (
    isBubbleOpen && (
      <Flex
        style={{
          flexDirection: "column",
          position: "fixed",
          height: 0,
          width: "100%",
          alignSelf: "center",
          top: (deltaY ?? 0) + (down ? pos.bottom : pos.top),
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
          {down && (
            <Flex
              style={{
                width: 0,
                height: 0,
                borderLeft: "15px solid transparent",
                borderRight: "15px solid transparent",
                borderBottom: `15px solid ${arrowColor}`,
                alignSelf: "center",
                position: "relative",
                opacity: isPointing ? 1 : 0,
                left: centX,
              }}
            />
          )}
          <TourBox style={bubbleContainerStyles}>
            <TourTitle title={text} />
            {children}
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "8px",
                top: "1px",
                color: appearance == "light" ? "white" : "black",
              }}
              onClick={() => {
                dispatch(close());
              }}
            >
              ×
            </Link>
            <Link
              style={{
                cursor: "pointer",
                position: "absolute",
                right: "10px",
                bottom: "10px",
                textTransform: "uppercase",
                fontSize: "12px",
                fontWeight: "bold",
                color: appearance == "light" ? "#54a1ff" : "#004069",
              }}
              onClick={() => {
                dispatch(next());
                if (onNext) onNext();
              }}
            >
              next
            </Link>
          </TourBox>
          {down || (
            <Flex
              style={{
                width: 0,
                height: 0,
                borderLeft: "15px solid transparent",
                borderRight: "15px solid transparent",
                borderTop: `15px solid ${arrowColor}`,
                alignSelf: "center",
                position: "relative",
                opacity: isPointing ? 1 : 0,
                left: centX,
              }}
            />
          )}
        </Flex>
      </Flex>
    )
  );
}
