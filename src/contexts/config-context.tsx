// TODO: delete this
// import React, { useContext, createContext, useEffect } from "react";
// import { type Config, isUpdateConfigMessage } from "../events";

// export type { Config };

// const ConfigContext = createContext<Config>({
//   host: "web",
//   features: {
//     statistics: false,
//     vecdb: false,
//     ast: false,
//   },
// });

// // TODO: add theme props, and configure vscode to grey
// const ConfigProvider: React.FC<{
//   children: React.ReactNode;
//   config: Config;
// }> = ({ children, config: configFromHost }) => {
//   const [config, setConfig] = React.useState<Config>(configFromHost);

//   useEffect(() => {
//     const listener = (event: MessageEvent) => {
//       if (isUpdateConfigMessage(event.data)) {
//         const { payload } = event.data;
//         setConfig((prev) => {
//           const nextConfig: Config = {
//             ...prev,
//             ...payload,
//             features: {
//               ...prev.features,
//               ...payload.features,
//             },
//           };
//           return nextConfig;
//         });
//       }
//     };

//     window.addEventListener("message", listener);

//     return () => window.removeEventListener("message", listener);
//   }, [setConfig]);

//   return (
//     <ConfigContext.Provider value={config}>{children}</ConfigContext.Provider>
//   );
// };

// const useConfig = () => useContext(ConfigContext);

// // eslint-disable-next-line react-refresh/only-export-components
// export { ConfigProvider, useConfig };
