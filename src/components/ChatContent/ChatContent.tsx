import React, { useEffect } from "react";
import { ChatMessages } from "../../services/refact";
import { Markdown } from "../Markdown";

const UserInput: React.FC<{ children: string }> = (props) => {
  return <Markdown>{props.children}</Markdown>;
};

const ContextFile: React.FC<{ children: string }> = (props) => {
  return <Markdown>{props.children}</Markdown>;
};

const ChatInput: React.FC<{ children: string }> = (props) => {
  return <Markdown>{props.children}</Markdown>;
};

export const ChatContent: React.FC<{ messages: ChatMessages }> = ({
  messages,
}) => {
  const ref = React.useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollIntoView && ref.current.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages]);

  return (
    <div
      style={{
        flexGrow: 1,
        overflowY: "auto",
        overflowX: "auto",
        wordWrap: "break-word",
      }}
    >
      {messages.map(([role, text], index) => {
        if (role === "user") {
          return <UserInput key={index}>{text}</UserInput>;
        } else if (role === "context_file") {
          return <ContextFile key={index}>{text}</ContextFile>;
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
        } else if (role === "assistant") {
          return <ChatInput key={index}>{text}</ChatInput>;
        } else {
          return <Markdown key={index}>{text}</Markdown>;
        }
      })}
      <div ref={ref} />
    </div>
  );
};
