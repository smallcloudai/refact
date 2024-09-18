declare global {
  interface Window {
    postIntellijMessage?: (message: Record<string, unknown>) => void;
    acquireVsCodeApi?(): {
      postMessage: (message: Record<string, unknown>) => void;
    };
  }
}

let postMessage: Window["postMessage"] | undefined;

function setUpPostMessage() {
  if (postMessage) return postMessage;
  if (window.acquireVsCodeApi) {
    postMessage = window.acquireVsCodeApi().postMessage;
  } else if (window.postIntellijMessage) {
    postMessage = window.postIntellijMessage;
  } else {
    postMessage = (message: Record<string, unknown>) =>
      window.postMessage(message, "*");
  }
  return postMessage;
}

export const usePostMessage = () => {
  return setUpPostMessage();
};
