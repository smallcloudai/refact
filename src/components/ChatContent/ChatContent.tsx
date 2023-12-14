import React, { useEffect } from 'react'
import { ChatMessages } from '../../services/refact'
import { Markdown } from '../Markdown'

const UserInput: React.FC<{children: string}> = (props) => {
    return <Markdown>{props.children}</Markdown>
}

const ContextFile: React.FC<{children: string}> = (props) => {
    return <Markdown>{props.children}</Markdown>
}

const ChatInput: React.FC<{children: string}> = (props) => {
    return <Markdown>{props.children}</Markdown>
}


export const ChatContent: React.FC<{messages: ChatMessages}> = ({messages}) => {
    const ref = React.useRef<HTMLDivElement>(null)
    useEffect(() => {
        const  box = ref.current?.getBoundingClientRect();
        box && ref.current?.scrollTo({
            left: box.left,
            top: box.bottom,
            behavior:'smooth'
        })
    }, [messages])

    return (<div ref={ref} style={{
        flexGrow: 1,
        overflowY: "auto",
        overflowX: "auto",
        wordWrap: "break-word",
    }}>
        {messages.map(([role, text], index) => {
            if(role === 'user') {
                return <UserInput key={index}>{text}</UserInput>
            } else if(role === "context_file") {
                return <ContextFile key={index}>{text}</ContextFile>
            // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
            } else if(role === "assistant") {
                return <ChatInput key={index}>{text}</ChatInput>
            } else {
                return <Markdown key={index}>{text}</Markdown>
            }
        })}
    </div>)
}
