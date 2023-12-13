const REFACT_URL = "http://127.0.0.1:8001";
const CHAT_URL = `${REFACT_URL}/v1/chat`;

export type ChatRole = "user" | "assistant" | "context_file"; // string; // TODO: narrow this typing
export type ChatMessage = [ChatRole, string];
export type ChatMessages = ChatMessage[];

interface BaseDelta {
  role: ChatRole;
}

interface UserDelta extends BaseDelta {
  role: "user";
  content: string;
}

interface AssistantDelta extends BaseDelta {
  role: "assistant";
  content: string;
}

interface ChatContextFile extends BaseDelta {
  role: "context_file";
  file_content: string;
}

type Delta = UserDelta | AssistantDelta | ChatContextFile
// interface Delta extends UserDelta, AssistantDelta , ChatContextFile {}


export type ChatChoice = {
  delta: Delta;
  finish_reason: "stop" | "abort" | null;
  index: number;
}
export type ChatResponse = {
  choices: ChatChoice[];
  created: number;
  model: string;
}

// const messages = [
//   {role: 'user', content: 'what is a shape file?'},
//   {role: 'assistant', content: 'A shapefile is a common geospatial vector data format used in Geographic Information System (GIS) software. It stores geometric and attribute data for geographic features, such as points, lines, and polygons. Shapefiles consist of multiple files with extensions like .shp, .shx, and .dbf. You can find more information about shapefiles in the official documentation: https://en.wikipedia.org/wiki/Shapefile'},
//   {role: 'user', content: 'how do I modify a shape file'},
//   {role: 'assistant', content: 'To modify a shapefile, you can use GIS software or programming libraries that support shapefile editing. Here are a few common approaches:\n\n1. GIS Software: Use software like ArcGIS, QGIS, or MapInfo, which provide user-friendly interfaces to edit shapefiles. These tools allow you to add, delete, or modify features, as well as edit attribute data associated with the shapefile.\n\n2. Programming Libraries: Utilize programming languages like Python with libraries such as geopandas, shapely, or pysh…methods to read, write, and modify shapefiles programmatically. You can manipulate the geometry and attribute data using these libraries.\n\nHere are some resources to get started with shapefile editing using programming libraries:\n- geopandas documentation: https://geopandas.org/\n- shapely documentation: https://shapely.readthedocs.io/\n- pyshp documentation: https://github.com/GeospatialPython/pyshp\n\nRemember to make a backup of your shapefile before making any modifications to avoid data loss.'},
//   {role: 'user', content: 'how do i pragmatically modify a shape file?'}
// ]

const API_KEY: string | undefined = import.meta.env.VITE_REFACT_API_KEY;
if (!API_KEY) {
  console.error("REFACT_API_KEY not configured")
  throw new Error("api-key not defined");
}


export function sendChat(
  messages: ChatMessages,
  model: string,
  abortController: AbortController,
) {

  const jsonMessages = messages.map(([role, content]) => {
    return { role, content };
  });

  const body = JSON.stringify({
    messages: jsonMessages,
    model: model,
    parameters: {
      max_new_tokens: 1000,
    },
    stream: true,
  });

  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${API_KEY}`,
  };

  return fetch(CHAT_URL, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    signal: abortController.signal
  })
  // .then(res => res.body)

}
