import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";

// import './index.css'

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
ReactDOM.createRoot(document.getElementById("refact-chat")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
