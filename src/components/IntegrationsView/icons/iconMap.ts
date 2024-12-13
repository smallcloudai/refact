import terminalIcon from "./cmdline.png";
import postgresIcon from "./postgres.png";
import dockerIcon from "./docker.png";
import chromeIcon from "./chrome.png";
import pdbIcon from "./pdb.png";
import githubLightIcon from "./github-light.png";
import githubDarkIcon from "./github-dark.png";
import gitlabIcon from "./gitlab.png";
import mysqlIcon from "./mysql.png";

export const iconMap = (theme: "light" | "dark"): Record<string, string> => {
  if (theme === "light") {
    return {
      cmdline: terminalIcon,
      postgres: postgresIcon,
      mysql: mysqlIcon,
      docker: dockerIcon,
      isolation: dockerIcon,
      chrome: chromeIcon,
      pdb: pdbIcon,
      github: githubLightIcon,
      gitlab: gitlabIcon,
      shell: terminalIcon,
    };
  }

  return {
    cmdline: terminalIcon,
    postgres: postgresIcon,
    mysql: mysqlIcon,
    docker: dockerIcon,
    isolation: dockerIcon,
    chrome: chromeIcon,
    pdb: pdbIcon,
    github: githubDarkIcon,
    gitlab: gitlabIcon,
    shell: terminalIcon,
  };
};
