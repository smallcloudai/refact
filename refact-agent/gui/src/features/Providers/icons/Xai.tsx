import { FC, SVGProps } from "react";

export const XaiIcon: FC<SVGProps<SVGSVGElement>> = (props) => {
  return (
    <svg
      width="30"
      height="30"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
      fill="currentColor"
      {...props}
    >
      <path d="M2.30047 8.77631L12.0474 23H16.3799L6.63183 8.77631H2.30047ZM6.6285 16.6762L2.29492 23H6.63072L8.79584 19.8387L6.6285 16.6762ZM17.3709 1L9.88007 11.9308L12.0474 15.0944L21.7067 1H17.3709ZM18.1555 7.76374V23H21.7067V2.5818L18.1555 7.76374Z"></path>
    </svg>
  );
};
