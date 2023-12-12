import React from "react";
import {IconButton} from '@radix-ui/themes'
import { PaperPlaneIcon, ExitIcon} from '@radix-ui/react-icons';


type IconButtonProps =  React.ComponentProps<typeof IconButton>

export const PaperPlaneButton: React.FC<IconButtonProps> = (props) => (
    <IconButton variant="ghost" {...props}>
        <PaperPlaneIcon />
    </IconButton>
)

export const BackToSideBarButton: React.FC<IconButtonProps> = (props) => (
    <IconButton variant="ghost" {...props}>
        <ExitIcon style={{transform: "scaleX(-1)"}} />
    </IconButton>
)