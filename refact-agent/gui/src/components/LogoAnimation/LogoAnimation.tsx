import React, { useEffect, useMemo } from "react";
import {
  useLottie,
  type LottieOptions,
  type LottieComponentProps,
} from "lottie-react";
import logoAnimationData from "./animationData.json";
import { defaultSize, type AnimationSize } from "./types";

export type LogoAnimationProps = Omit<
  LottieComponentProps,
  "animationData" | "size"
> & {
  size?: AnimationSize;
  isWaiting: boolean;
  isStreaming: boolean;
};

function sizeToCssVariable(size: AnimationSize) {
  const sizeString = `var(--font-size-${size}, calc(16px * var(--scaling, 1))`;
  return { width: sizeString, height: sizeString };
}

const EYE_ANIMATION_FRAMES: [number, number] = [0, 70];
const CHAR_ANIMATION_FRAMES: [number, number] = [79, 213];
const SPIN_ANIMATION_FRAMES: [number, number] = [214, 254];
const HAPPY_EYES: [number, number] = [242, 250];

function selectFrames(
  isWaiting: boolean,
  isStreaming: boolean,
): Pick<LottieOptions, "loop" | "initialSegment"> {
  if (isStreaming) {
    return {
      loop: true,
      initialSegment: CHAR_ANIMATION_FRAMES,
    };
  } else if (isWaiting) {
    return {
      loop: true,
      initialSegment: EYE_ANIMATION_FRAMES,
    };
  }
  return {
    loop: 1,
    initialSegment: HAPPY_EYES,
  };
}

export const LogoAnimation: React.FC<LogoAnimationProps> = ({
  size = defaultSize,
  isWaiting,
  isStreaming,
  ...props
}) => {
  const options: LottieOptions = useMemo(() => {
    const sizeProps = sizeToCssVariable(size);
    const styleProps = { ...sizeProps, ...props.style };
    return {
      ...props,
      style: styleProps,
      animationData: logoAnimationData,
      ...selectFrames(isWaiting, isStreaming),
    };
  }, [isStreaming, isWaiting, props, size]);

  const { View, playSegments } = useLottie(options);
  useEffect(() => {
    if (isWaiting && !isStreaming) {
      playSegments([EYE_ANIMATION_FRAMES, SPIN_ANIMATION_FRAMES]);
    }
  }, [isStreaming, isWaiting, playSegments]);

  return View;
};
