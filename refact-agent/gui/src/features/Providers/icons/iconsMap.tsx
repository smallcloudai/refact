import { AnthropicIcon } from "./Anthropic";
import { DeepSeekIcon } from "./DeepSeek";
import { GeminiIcon } from "./Gemini";
import { GroqIcon } from "./Groq";
import { OpenAIIcon } from "./OpenAI";
import { OpenRouterIcon } from "./OpenRouter";
import { RefactIcon } from "./Refact";

export const iconsMap: Record<string, JSX.Element> = {
  refact: <RefactIcon />,
  refact_self_hosted: <RefactIcon />,
  openai: <OpenAIIcon />,
  anthropic: <AnthropicIcon />,
  google_gemini: <GeminiIcon />,
  openrouter: <OpenRouterIcon />,
  deepseek: <DeepSeekIcon />,
  groq: <GroqIcon />,
};
