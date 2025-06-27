import React from "react";

export const BranchIcon: React.FC<React.SVGAttributes<SVGElement>> = (
  props,
) => (
  <svg
    width="15px"
    height="15px"
    viewBox="0 0 256 256"
    xmlns="http://www.w3.org/2000/svg"
    fill="none"
    style={{
      transform: "rotate(180deg)",
    }}
    {...props}
  >
    <path
      d="M68,160v-8a23.9,23.9,0,0,1,24-24h72a23.9,23.9,0,0,0,24-24V96"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="12"
    />
    <line
      color="currentColor"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="12"
      x1="68"
      x2="68"
      y1="96"
      y2="160"
    />
    <circle
      cx="68"
      cy="188"
      stroke="currentColor"
      r="28"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="12"
    />
    <circle
      cx="188"
      cy="68"
      stroke="currentColor"
      r="28"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="12"
    />
    <circle
      cx="68"
      cy="68"
      r="28"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="12"
    />
  </svg>
);
