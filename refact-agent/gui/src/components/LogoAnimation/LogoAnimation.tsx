import React from "react";
import Lottie, { type LottieComponentProps } from "lottie-react";
import logoAnimationData from "./animationData.json";
import { defaultSize, type AnimationSize } from "./types";

export type LogoAnimationProps = Omit<LottieComponentProps, "animationData"> & {
  size?: AnimationSize;
};

function sizeToCssVariable(size: AnimationSize) {
  const sizeString = `var(--font-size-${size}, calc(16px * var(--scaling, 1))`;
  return { width: sizeString, height: sizeString };
}

export const LogoAnimation: React.FC<LogoAnimationProps> = ({
  size = defaultSize,
  ...props
}) => {
  const sizeProps = sizeToCssVariable(size);
  const styleProps = { ...sizeProps, ...props.style };
  return (
    <Lottie
      animationData={logoAnimationData}
      loop={true}
      {...props}
      style={styleProps}
    />
  );
};
