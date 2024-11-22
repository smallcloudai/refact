import type { RootState } from "../app/store";
import { ChatHistoryItem } from "../features/History/historySlice";

export * from "./some_chrome_screenshots";

type ChatThread = RootState["chat"]["thread"];
type ChatMessages = ChatThread["messages"];

export const MARS_ROVER_CHAT: ChatHistoryItem = {
  id: "9afd6fef-3e49-40df-8aca-688af3621514",
  messages: [
    // [
    //   "system",
    //   "Use backquotes for code blocks. Pay close attention to indent when editing code blocks: indent must be exactly the same as in the original code block.",
    // ],
    { role: "user", content: "mars rover kata" },
    {
      role: "context_file",
      content: [
        {
          file_name: "file/name.txt",
          file_content: "hello",
          line1: 1,
          line2: 2,
          usefulness: 100,
        },
      ],
    },
    {
      role: "assistant",
      content:
        "The Mars Rover Kata is a coding exercise that simulates the movement of a rover on the surface of Mars. The goal is to write a program that takes in a series of commands and outputs the final position of the rover.\n\nHere is a link to the official Mars Rover Kata documentation: https://kata-log.rocks/mars-rover-kata\n\nYou can find various implementations and solutions to the Mars Rover Kata on platforms like GitHub or coding challenge websites.",
    },
    { role: "user", content: "solve in ocaml" },
    {
      role: "assistant",
      content:
        "Certainly! Here's an example solution to the Mars Rover Kata in OCaml:\n\n```ocaml\ntype direction = North | East | South | West\n\ntype rover = {\n  x : int;\n  y : int;\n  dir : direction;\n}\n\nlet move_forward rover =\n  match rover.dir with\n  | North -> { rover with y = rover.y + 1 }\n  | East -> { rover with x = rover.x + 1 }\n  | South -> { rover with y = rover.y - 1 }\n  | West -> { rover with x = rover.x - 1 }\n\nlet turn_left rover =\n  match rover.dir with\n  | North -> { rover with dir = West }\n  | East -> { rover with dir = North }\n  | South -> { rover with dir = East }\n  | West -> { rover with dir = South }\n\nlet turn_right rover =\n  match rover.dir with\n  | North -> { rover with dir = East }\n  | East -> { rover with dir = South }\n  | South -> { rover with dir = West }\n  | West -> { rover with dir = North }\n\nlet execute_command rover command =\n  match command with\n  | 'F' -> move_forward rover\n  | 'L' -> turn_left rover\n  | 'R' -> turn_right rover\n  | _ -> rover (* Ignore unknown commands *)\n\nlet rec execute_commands rover commands =\n  match commands with\n  | [] -> rover\n  | command :: rest ->\n    let new_rover = execute_command rover command in\n    execute_commands new_rover rest\n\nlet initial_rover = { x = 0; y = 0; dir = North }\n\nlet final_rover = execute_commands initial_rover ['F'; 'R'; 'F'; 'F'; 'L'; 'F']\n\nlet () =\n  Printf.printf \"Final position: (%d, %d), facing %s\\n\"\n    final_rover.x final_rover.y\n    (match final_rover.dir with\n    | North -> \"North\"\n    | East -> \"East\"\n    | South -> \"South\"\n    | West -> \"West\")\n```\n\nThis solution defines the necessary types, functions, and commands to simulate the movement of the rover. The `execute_commands` function takes in a rover and a list of commands and iteratively executes each command to update the rover's position and direction. Finally, the `final_rover` is printed to the console.\n\nYou can run this code in an OCaml environment or compile it using an OCaml compiler.",
    },

    // remark rehype-katex works with this
    { role: "user", content: "the quadratic formula." },
    {
      role: "assistant",

      // "$$ \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a} $$",
      // "The quadratic formula is given by:\n\n\\[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} \\]\n\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
      content:
        "The quadratic formula is given by:$$[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} ] $$You can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
    },
    { role: "user", content: "formalla with new lines" },
    {
      role: "assistant",
      content:
        // "$$ \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a} $$",
        // "The quadratic formula is given by:\n\n\\[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} \\]\n\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
        "The quadratic formula is given by:\n$$\nx = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}}\n$$\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
    },
    { role: "user", content: "other math" },
    {
      role: "assistant",
      // "$$ \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a} $$",
      // "The quadratic formula is given by:\n\n\\[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} \\]\n\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
      content: "block\n\n```math\nC_L\n```\n\ninline: $C_L$\n\n",
    },
    { role: "user", content: "long message\n" + "a".repeat(10000) },
  ],
  title: "mars rover kata",
  isTitleGenerated: true,
  model: "gpt-3.5-turbo",
  tool_use: "explore",
  createdAt: "2023-12-21T17:32:50.186Z",
  updatedAt: "2023-12-21T17:33:22.209Z",
};

export const CHAT_FUNCTIONS_MESSAGES: ChatMessages = [
  {
    role: "system",
    content:
      '\nYou are a search agent. You need to actively search for the answer yourself, don\'t ask the user to do anything. The answer is most likely in the files and databases accessible using tool calls, not on the internet.\n\nWhen responding to a query, first provide a very brief explanation of your plan to use tools in parallel to answer the question, and then make several tool calls to gather more details.\n\nMinimize the number of steps, call up to 15 tools in parallel when exploring.\n\nIT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST!\n\nWhen user corrects you, acknowledge the correction, write "I will make a note to remember this" and use note_to_self call.\n\n\nExample 1\n\nUser: "What is the weather like today in Paris and London?"\nAssistant: "Must be sunny in Paris and foggy in London."\nUser: "don\'t hallucinate, use the tools"\nAssistant: "Sorry for the confusion, you are right, weather is real-time, and my best shot is to use the weather tool. I will make a note to remember this. My original instruction was to fetch weather for Paris and London. I will use 2 calls in parallel."\n[Call note_to_self "Weather" "For weather, use tools to get real-time information"]\n[Call weather "London"]\n[Call weather "Paris"]\n\n\nExample 2\n\nUser: "What is MyClass"\nAssistant: "Let me find it first."\n[Call ls "."]\nTool: subdir1, subdir2, subdir3\nAssistant: "I see 3 subdirs, will make 3 calls in parallel to check what\'s inside."\n[Call ls "subdir1"]\n[Call ls "subdir2"]\n[Call ls "subdir3"]\nTool: ...\nTool: ...\nTool: ...\nAssistant: "I give up, I can\'t find a file relevant for MyClass 😕"\nUser: "Look, it\'s my_class.cpp"\nAssistant: "Sorry for the confusion, there is in fact a file named `my_class.cpp` in `subdir2` that must be relevant for MyClass. I will make a note to remember this. My original instruction was to describe MyClass."\n[Call note_to_self "searching, browsing filesystem, MyClass" "Try to lowercase the name user asked about, convert to snake case, and guess the right file this way."]\n[Call cat "subdir2/my_class.cpp"]\nTool: ...\nAssistant: "MyClass does this and this"\n\n\n',
  },
  { role: "user", content: "Explain what Frog is" },
  {
    role: "assistant",
    content:
      "Let me find the relevant information about Frog first. I will check the contents of files or directories that might contain information about Frog. I will start by listing the files in the current directory to see if there are any relevant files or directories.",
  },
  {
    role: "assistant",
    content:
      "Let me find the relevant information about Frog first. I will check the contents of files or directories that might contain information about Frog. I will start by listing the files in the current directory to see if there are any relevant files or directories.",
    tool_calls: [
      {
        id: "call_WOyQ1sykVGppzWjjUu1drk6L",
        function: {
          arguments: '{"dirpath":"."}',
          name: "ls",
        },
        type: "function",
        index: 0,
      },
    ],
  },
  // TODO: this might not be correct
  {
    role: "tool",
    content: {
      tool_call_id: "call_WOyQ1sykVGppzWjjUu1drk6L",
      content:
        "Listing directory .\n      2260    file Cargo.toml\n      1530    file LICENSE\n       224     dir target\n      1198    file mycaps_te3.json\n       416     dir tests\n    152298    file Cargo.lock\n       757    file mycaps_openai.json\n        61    file build.rs\n      1264    file mycaps_gte.json\n      1598    file _video\n      3548    file README.md\n       768     dir examples\n       219    file _backtrace\n      1665    file _video2\n       141    file a.sh\n       139    file _help\n       992     dir src\n",
      finish_reason: "call_worked",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_IYK970zyp9vZ36m7emzmNDC9",
      content:
        'File README.md:50-99\n```    "temperature": 0.1,\n    "max_new_tokens": 20\n  }\n}\'\n```\n\nOutput is `[{"code_completion": "\\n    return \\"Hello World!\\"\\n"}]`.\n\n[LSP example](examples/lsp_completion.py)\n\n\n## Telemetry\n\nThe flags `--basic-telemetry` and `--snippet-telemetry` control what telemetry is sent. To be clear: without\nthese flags, no telemetry is sent. Those flags are typically controlled from IDE plugin settings.\n\nBasic telemetry means counters and error messages without information about you or your code. It is "compressed"\ninto `.cache/refact/telemetry/compressed` folder, then from time to time it\'s sent and moved\nto `.cache/refact/telemetry/sent` folder.\n\n"Compressed" means similar records are joined together, increasing the counter. "Sent" means the rust binary\ncommunicates with a HTTP endpoint specified in caps (see Caps section below) and sends .json file exactly how\nyou see it in `.cache/refact/telemetry`. The files are human-readable.\n\nWhen using Refact self-hosted server, telemetry goes to the self-hosted server, not to the cloud.\n\n\n## Caps File\n\nThe `--address-url` parameter controls the behavior of this program by a lot. The address is first used\nto construct `$URL/coding_assistant_caps.json` address to fetch the caps file. Furthermore, there are\ncompiled-in caps you can use by magic addresses "Refact" and "HF".\n\nThe caps file describes which models are running, default models for completion and chat,\nwhere to send the telemetry, how to download a\ntokenizer, where is the endpoint to access actual language models. To read more, check out\ncompiled-in caps in [caps.rs](src/caps.rs).\n\n\n## Tests\n\nThe one to run often is [test_edge_cases.py](tests/test_edge_cases.py).\n\nYou can also run [measure_humaneval_fim.py](tests/measure_humaneval_fim.py) for your favorite model.\n\n\n## Credits\n\nThe initial version of this project was written by looking at llm-ls by [@McPatate](https://github.com/McPatate). He\'s a Rust fan who inspired this project!\n```',
      finish_reason: "call_worked",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_8jTn7oj8tfctEnqgKQRBJH0w",
      content:
        'File Cargo.toml:39-88\n```futures-util = "0.3"\nasync-stream = "0.3.5"\nchrono = "0.4.31"\nregex = "1.9.5"\nasync-trait = "0.1.73"\nsimilar = "2.3.0"\naxum = "0.6.20"\nuuid = { version = "1", features = ["v4"] }\nlazy_static = "1.4.0"\n\nregex-automata = { version = "0.1.10", features = ["transducer"] }\nsorted-vec = "0.8.3"\ntree-sitter = "0.20"\ntree-sitter-cpp = "0.20"\n#tree-sitter-c-sharp = "0.20"\ntree-sitter-java = "0.20"\ntree-sitter-javascript = "0.20"\n#tree-sitter-kotlin = "0.3.1"\ntree-sitter-python = "0.20"\ntree-sitter-rust = "0.20"\ntree-sitter-typescript = "0.20"\n\narrow = "47.0.0"\narrow-array = "47.0.0"\narrow-schema= "47.0.0"\nasync_once= "0.2.6"\nasync-process = "2.0.1"\nitertools = "0.11.0"\nlance = "=0.9.0"\nlance-linalg = "=0.9.0"\nlance-index = "=0.9.0"\nlog = "0.4.20"\nmd5 = "0.7"\nmockito = "0.28.0"\nnotify = { version = "6.1.1", features = ["serde"] }\nparking_lot = { version = "0.12.1", features = ["serde"] }\nrusqlite = { version = "0.30.0", features = ["bundled"] }\ntempfile = "3.8.1"\ntime = "0.3.30"\ntokio-rusqlite = "0.5.0"\nvectordb = "=0.4.0"\nwalkdir = "2.3"\nwhich = "5.0.0"\nstrsim = "0.8.0"\ntypetag = "0.2"\ndyn_partial_eq = "=0.1.2"\nrayon = "1.8.0"\nbacktrace = "0.3.71"\nrand = "0.8.5"\n```',
      finish_reason: "call_worked",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_Ql7xrkn5BqtjVSHHAnNksFis",
      content:
        'File Cargo.lock:6265-6314\n```]\n\n[[package]]\nname = "zstd"\nversion = "0.11.2+zstd.1.5.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "20cc960326ece64f010d2d2107537f26dc589a6573a316bd5b1dba685fa5fde4"\ndependencies = [\n "zstd-safe 5.0.2+zstd.1.5.2",\n]\n\n[[package]]\nname = "zstd"\nversion = "0.12.4"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "1a27595e173641171fc74a1232b7b1c7a7cb6e18222c11e9dfb9888fa424c53c"\ndependencies = [\n "zstd-safe 6.0.6",\n]\n\n[[package]]\nname = "zstd-safe"\nversion = "5.0.2+zstd.1.5.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "1d2a5585e04f9eea4b2a3d1eca508c4dee9592a89ef6f450c11719da0726f4db"\ndependencies = [\n "libc",\n "zstd-sys",\n]\n\n[[package]]\nname = "zstd-safe"\nversion = "6.0.6"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "ee98ffd0b48ee95e6c5168188e44a54550b1564d9d530ee21d5f0eaed1069581"\ndependencies = [\n "libc",\n "zstd-sys",\n]\n\n[[package]]\nname = "zstd-sys"\nversion = "2.0.9+zstd.1.5.5"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "9e16efa8a874a0481a574084d34cc26fdb3b99627480f785888deb6386506656"\ndependencies = [\n "cc",\n "pkg-config",\n]\n```',
      finish_reason: "call_worked",
    },
  },
  {
    role: "assistant",
    content:
      "Based on the information found in the files:\n\n- Frog seems to be a project related to coding assistance and telemetry.\n- Telemetry is sent based on flags like `--basic-telemetry` and `--snippet-telemetry`.\n- The `Caps File` contains information about models, default models for completion and chat, telemetry destinations, tokenizers, and language models.\n- Tests related to the project include `test_edge_cases.py` and `measure_humaneval_fim.py`.\n- The project credits [@McPatate](https://github.com/McPatate) for the initial version, who is a Rust fan.\n\nFor more specific details about Frog, further exploration into the project's source code or documentation may be needed.",
  },
  { role: "user", content: "continue searching for the Frog in tests" },
  {
    role: "tool",
    content: {
      tool_call_id: "call_KKgK2Ki3bpvCk2jCtDhQxfpw",
      content:
        "Listing directory tests\n      2438    file test_hf_endpoint.py\n      3021    file lsp_connect.py\n       678    file lsp_completion.py\n         0    file __init__.py\n        96     dir __pycache__/\n      1340    file test_at_completion.py\n      3926    file post_doc_info.py\n       224     dir emergency_frog_situation/\n      4107    file test_edge_cases.py\n      2081    file code_completion_with_rag.py\n      2866    file measure_humaneval_fim.py\n",
      finish_reason: "call_worked",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_dT0OdIL7JLYJ7Fxk72MvmuRA",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_C5uTWek5PUKmaTfe7u0TypZL",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_9vYcfrFCiUhy8g6bNn0WJn5p",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_6Cg6UfAvNTgEt96EQrHz16W6",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  },
  {
    role: "assistant",
    content:
      "I couldn't retrieve specific information about Frog in the tests directory. If there are any other locations or specific files you suspect might contain details about Frog, please let me know.",
  },
  { role: "user", content: "use ls, don't be stupid" },
  {
    role: "tool",
    content: {
      tool_call_id: "call_UoHvkwbPq6LMAKRM0iblVkSB",
      content:
        "Listing directory tests/emergency_frog_situation\n      1516    file jump_to_conclusions.py\n       695    file set_as_avatar.py\n        96     dir __pycache__/\n       777    file frog.py\n       249    file work_day.py\n",
      finish_reason: "call_worked",
    },
  },
  {
    role: "assistant",
    content:
      "In the `tests/emergency_frog_situation` directory, the file `frog.py` seems to be relevant to Frog. Let's check the content of `frog.py` to find out more about Frog.",
  },
  {
    role: "assistant",
    content:
      "In the `tests/emergency_frog_situation` directory, the file `frog.py` seems to be relevant to Frog. Let's check the content of `frog.py` to find out more about Frog.",
    tool_calls: [
      {
        id: "call_spx7e7LMfw97BmmzojQQf0rO",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation/frog.py"}',
          name: "cat",
        },
        type: "function",
        index: 0,
      },
    ],
  },
  {
    role: "tool",
    content: {
      tool_call_id: "call_spx7e7LMfw97BmmzojQQf0rO",
      content:
        "File tests/emergency_frog_situation/frog.py:1-29\n```import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n```",
      finish_reason: "call_worked",
    },
  },
  {
    role: "assistant",
    content:
      "The file `frog.py` in the `tests/emergency_frog_situation` directory contains a Python class `Frog` with methods related to simulating the movement of a frog in a pond. The class includes methods for initialization, bouncing off banks, and jumping within the pond. The `Frog` class has attributes like position (`x`, `y`) and velocity (`vx`, `vy`), along with methods to handle movement and boundary conditions.\n\nThis information suggests that Frog in this context is related to a simulation or program involving a frog's behavior in a pond.",
  },
];

export const FROG_CHAT: ChatThread = {
  id: "77b6a451-5598-44c0-bd5b-cfc19e3f4e60",
  tool_use: "explore",
  messages: [
    {
      role: "user",
      content: "\nWhat is the difference between a frog and a toad?\n",
    },
    {
      role: "assistant",
      content: null,
      tool_calls: [
        {
          function: {
            arguments: '{"path": "frog.txt"}',
            name: "file",
          },
          id: "call_NSSpdvLovaH50zZUug463YRI",
          index: 0,
          type: "function",
        },
        {
          function: {
            arguments: '{"path": "toad.txt"}',
            name: "file",
          },
          id: "call_cmTkaNJ0roopnMcNfG4raxny",
          index: 1,
          type: "function",
        },
      ],
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_NSSpdvLovaH50zZUug463YRI",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_cmTkaNJ0roopnMcNfG4raxny",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    },
    {
      role: "context_file",
      content: [
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
      ],
    },
    {
      role: "assistant",
      content: null,
      tool_calls: [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/toad.py"}',
            name: "file",
          },
          id: "call_8ER9PVREdkt37h84LZyc97c9",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_8ER9PVREdkt37h84LZyc97c9",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    },
    {
      role: "context_file",
      content: [
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
      ],
    },
    {
      role: "assistant",
      content:
        'The content of the files "frog.py" and "toad.py" seems to be the same. It appears that there was an issue with retrieving the correct content for the "toad.py" file. Let me attempt to fetch the content of the "toad.py" file again.',
      tool_calls: [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/toad.py"}',
            name: "file",
          },
          id: "call_1bHhD3bVIzvOueSDq1otYX4i",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_1bHhD3bVIzvOueSDq1otYX4i",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    },
    {
      role: "context_file",
      content: [
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
      ],
    },
    {
      role: "assistant",
      content:
        'It seems that there was an issue with retrieving the content of the "toad.py" file. Since the content of the "frog.py" file has been successfully retrieved, I will analyze the content and provide the differences between a frog and a toad based on the code.\nBased on the content of the "frog.py" file, it defines a class named "Frog" with methods for initializing the frog\'s position and velocity, bouncing off the banks of a pond, and jumping within the pond. The use of the numpy library for mathematical operations is also evident.\n\nHowever, since the content of the "toad.py" file was not retrieved successfully, I am unable to provide the specific differences between a frog and a toad based on the code. It seems that there might be an issue with the file retrieval process. If there are specific characteristics or differences you would like to know about frogs and toads, please let me know, and I can attempt to gather the information from other sources.',
      tool_calls: null,
    },
  ],
  title: "",
  model: "",
};

export const CHAT_WITH_DIFFS: ChatMessages = [
  {
    role: "assistant",
    content:
      "Persistence is essential in software development to ensure that data is stored and maintained even after the application is closed or the system is shut down.",
    tool_calls: null,
  },
  {
    role: "context_file",
    content: [
      {
        file_name:
          "hibernate-orm/hibernate-core/src/test/java/org/hibernate/orm/test/id/usertype/UserTypeComparableIdTest.java",
        line1: 1,
        line2: 228,
        file_content:
          "/*\\n * Hibernate, Relational Persistence for Idiomatic Java\\n *\\n * License: GNU Lesser General Public License (LGPL), version 2.1 or later.\\n * See the lgpl.txt",
      },
    ],
  },
  {
    role: "diff",
    content: [
      {
        file_name: "file1.py",
        file_action: "edit",
        line1: 5,
        line2: 6,
        lines_remove: "def f(x: int):\n    return x*2\n",
        lines_add: "def f(x: float):\n    return x*3\n",
      },
      {
        file_name: "file1.py",
        file_action: "edit",
        line1: 8,
        line2: 9,
        lines_remove: "def g(x: int):\n    return x*4\n",
        lines_add: "def g(x: float):\n    return x*5\n",
      },
      {
        file_name: "file2.py",
        file_action: "new",
        lines_add: "def main():\n    file1.f(6)\n",
        line1: 1,
        line2: 2,
        lines_remove: "",
      },
    ],
    tool_call_id: "test_tool_call_id",
  },
  { role: "user", content: "A longer diff" },
  { role: "assistant", content: "here you go" },
  {
    role: "diff",
    content: [
      {
        file_name: "long.py",
        file_action: "new",
        lines_add: "def main():    file1.f(6);    ".repeat(6),
        lines_remove: "",
        line1: 1,
        line2: 2,
      },
    ],
    tool_call_id: "test_tool_call_id",
  },
];

export const CHAT_WITH_DIFF_ACTIONS: ChatThread = {
  id: "eeda523e-9b74-4df6-8d60-a14ccdd907f0",
  messages: [
    {
      role: "user",
      content: "In the project add an edible property to the frog class\n",
    },
    {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          function: {
            arguments: '{"query":"class Frog"}',
            name: "search_workspace",
          },
          id: "call_n5qeQaFZNAoaP3qJzRiGO6Js",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_n5qeQaFZNAoaP3qJzRiGO6Js",
        content: "performed vecdb search, results below",
      },
    },
    {
      role: "context_file",
      content: [
        {
          file_content:
            '# Picking up context, goal in this file:\n# - goto parent class, two times\n# - dump parent class\n\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\n\n# This this a comment for the Toad class, above the class\nclass Toad(frog.Frog):\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "Bob"\n\n\nclass EuropeanCommonToad(frog.Frog):\n    """\n    This is a comment for EuropeanCommonToad class, inside the class\n    """\n\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "EU Toad"\n\n\nif __name__ == "__main__":\n    toad = EuropeanCommonToad(100, 100, 200, -200)\n    toad.jump(W, H)\n    print(toad.name, toad.x, toad.y)\n\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          line1: 1,
          line2: 32,
          usefulness: 0,
        },
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
        {
          file_content:
            '# Picking up context, goal in this file:\n# - pick up type of p\n# - prioritize type definition over all the noise\n\nimport pygame\nimport numpy as np\nimport frog\nfrom typing import Tuple\n\n\nW = 640\nH = 480\n\n\ndef draw_hello_frog(\n    screen: pygame.Surface,\n    message: str,\n    color: Tuple[int, int, int] = (0, 255, 255),\n    font_name: str = "Arial",\n) -> None:\n    font = pygame.font.SysFont(font_name, 32)\n    text = font.render(message, True, color)\n    text_rect = text.get_rect()\n    text_rect.center = (W / 2, H / 2)\n    screen.blit(text, text_rect)\n\n\ncreatures = [\n    frog.Frog(\n        np.random.uniform(0, W),\n        np.random.uniform(0, H),\n        np.random.uniform(-W/10, H/10),\n        np.random.uniform(-W/10, H/10),\n    ) for i in range(10)]\n\n\ndef main_loop():\n    screen = pygame.display.set_mode((W,H))   # without space because it\'s a test it needs to pick up right line below\n    quit_flag = False\n    while not quit_flag:\n        for event in pygame.event.get():\n            if event.type == pygame.QUIT:\n                quit_flag = True\n        screen.fill((0, 0, 0))\n        for p in creatures:\n            pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)\n        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))\n        pygame.display.flip()\n        pygame.time.Clock().tick(60)\n        p: frog.Frog\n        for p in creatures:\n            p.jump(W, H)\n\n\nif __name__ == \'__main__\':\n    pygame.init()\n    pygame.display.set_caption("Pond")\n    main_loop()\n    pygame.quit()\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          line1: 1,
          line2: 58,
          usefulness: 0,
        },
        {
          file_content:
            "# Picking up context, goal in this file:\n# - without any other information, find method usage in another file by text similarity\n\nimport numpy as np\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\ndef bring_your_own_frog_to_work_day(f):\n    f.jump(W, H)\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py",
          line1: 1,
          line2: 11,
          usefulness: 0,
        },
      ],
    },
    {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py","todo":"Add an \'edible\' property to the Frog class and initialize it in the constructor."}',
            name: "patch",
          },
          id: "call_dIXVNlzugvrPJvTF5G7n1YgK",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "diff",
      content: [
        {
          file_action: "edit",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 1,
          lines_add:
            "class Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n        self.edible = True",
          lines_remove:
            "class Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy",
        },
      ],
      tool_call_id: "test_tool_call_id",
    },
    {
      role: "assistant",
      content:
        "The `Frog` class has been updated to include an `edible` property.",
      tool_calls: null,
    },
  ],
  title: "In the project add an edible property to the frog class\n",
  model: "gpt-4o",
  tool_use: "explore",
  createdAt: "2024-07-05T09:10:29.523Z",
  updatedAt: "2024-07-05T09:10:37.322Z",
};

export const LARGE_DIFF: ChatThread = {
  id: "7ebafb62-a701-43a3-9bdd-fd1425b54536",
  messages: [
    { role: "user", content: "rename the frog class to bird." },
    {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          function: {
            arguments: "{}",
            name: "tree",
          },
          id: "call_b0ZalvpaQCZLGIHS0t4O3tH3",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_b0ZalvpaQCZLGIHS0t4O3tH3",
        content:
          "  \n    Users\n      marc\n        Projects\n          refact-lsp\n            tests\n              emergency_frog_situation\n                frog.py\n                holiday.py\n                jump_to_conclusions.py\n                set_as_avatar.py\n                work_day.py\n",
      },
    },
    {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          function: {
            arguments: '{"query":"class Frog"}',
            name: "search_workspace",
          },
          id: "call_YozL4pz5zNwdEaNWhdVQdcIF",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "tool",
      content: {
        tool_call_id: "call_YozL4pz5zNwdEaNWhdVQdcIF",
        content: "performed vecdb search, results below",
      },
    },
    {
      role: "context_file",
      content: [
        {
          file_content:
            '# Picking up context, goal in this file:\n# - goto parent class, two times\n# - dump parent class\n\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\n\n# This this a comment for the Toad class, above the class\nclass Toad(frog.Frog):\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "Bob"\n\n\nclass EuropeanCommonToad(frog.Frog):\n    """\n    This is a comment for EuropeanCommonToad class, inside the class\n    """\n\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "EU Toad"\n\n\nif __name__ == "__main__":\n    toad = EuropeanCommonToad(100, 100, 200, -200)\n    toad.jump(W, H)\n    print(toad.name, toad.x, toad.y)\n\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          line1: 1,
          line2: 32,
          usefulness: 0,
        },
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 26,
          usefulness: 0,
        },
        {
          file_content:
            '# Picking up context, goal in this file:\n# - pick up type of p\n# - prioritize type definition over all the noise\n\nimport pygame\nimport numpy as np\nimport frog\nfrom typing import Tuple\n\n\nW = 640\nH = 480\n\n\ndef draw_hello_frog(\n    screen: pygame.Surface,\n    message: str,\n    color: Tuple[int, int, int] = (0, 255, 255),\n    font_name: str = "Arial",\n) -> None:\n    font = pygame.font.SysFont(font_name, 32)\n    text = font.render(message, True, color)\n    text_rect = text.get_rect()\n    text_rect.center = (W / 2, H / 2)\n    screen.blit(text, text_rect)\n\n\ncreatures = [\n    frog.Frog(\n        np.random.uniform(0, W),\n        np.random.uniform(0, H),\n        np.random.uniform(-W/10, H/10),\n        np.random.uniform(-W/10, H/10),\n    ) for i in range(10)]\n\n\ndef main_loop():\n    screen = pygame.display.set_mode((W,H))   # without space because it\'s a test it needs to pick up right line below\n    quit_flag = False\n    while not quit_flag:\n        for event in pygame.event.get():\n            if event.type == pygame.QUIT:\n                quit_flag = True\n        screen.fill((0, 0, 0))\n        for p in creatures:\n            pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)\n        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))\n        pygame.display.flip()\n        pygame.time.Clock().tick(60)\n        p: frog.Frog\n        for p in creatures:\n            p.jump(W, H)\n\n\nif __name__ == \'__main__\':\n    pygame.init()\n    pygame.display.set_caption("Pond")\n    main_loop()\n    pygame.quit()\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          line1: 1,
          line2: 58,
          usefulness: 0,
        },
        {
          file_content:
            "# Picking up context, goal in this file:\n# - without any other information, find method usage in another file by text similarity\n\nimport numpy as np\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\ndef bring_your_own_frog_to_work_day(f):\n    f.jump(W, H)\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py",
          line1: 1,
          line2: 11,
          usefulness: 0,
        },
        {
          file_content:
            "import frog\n\n\nif __name__ == __main__:\n    frog1 = frog.Frog()\n    frog2 = frog.Frog()\n\n    # First jump\n    frog1.jump()\n    frog2.jump()\n\n    # Second jump\n    frog1.jump()\n    frog2.jump()\n\n    # Third jump\n    frog1.jump()\n    frog2.jump()\n\n    # Forth jump\n    frog1.jump()\n    frog2.jump()\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
          line1: 1,
          line2: 21,
          usefulness: 0,
        },
      ],
    },
    {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          function: {
            arguments:
              '{"paths":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py","symbols":"Frog","todo":"Rename the class Frog to Bird and update all its references accordingly."}',
            name: "patch",
          },
          id: "call_GViOZuuRlNT5tGK7RXQZxkhu",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "diff",
      content: [
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          file_action: "edit",
          line1: 5,
          line2: 6,
          lines_remove: "class Frog:\n",
          lines_add: "class Bird:\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          file_action: "edit",
          line1: 5,
          line2: 6,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          file_action: "edit",
          line1: 13,
          line2: 14,
          lines_remove: "class Toad(frog.Frog):\n",
          lines_add: "class Toad(bird.Bird):\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          file_action: "edit",
          line1: 19,
          line2: 20,
          lines_remove: "class EuropeanCommonToad(frog.Frog):\n",
          lines_add: "class EuropeanCommonToad(bird.Bird):\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          file_action: "edit",
          line1: 7,
          line2: 8,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          file_action: "edit",
          line1: 29,
          line2: 30,
          lines_remove: "    frog.Frog(\n",
          lines_add: "    bird.Bird(\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          file_action: "edit",
          line1: 50,
          line2: 51,
          lines_remove: "        p: frog.Frog\n",
          lines_add: "        p: bird.Bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py",
          file_action: "edit",
          line1: 5,
          line2: 6,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
          file_action: "edit",
          line1: 1,
          line2: 2,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
          file_action: "edit",
          line1: 5,
          line2: 7,
          lines_remove: "    frog1 = frog.Frog()\n    frog2 = frog.Frog()\n",
          lines_add: "    frog1 = bird.Bird()\n    frog2 = bird.Bird()\n",
        },
      ],
      tool_call_id: "call_GViOZuuRlNT5tGK7RXQZxkhu",
    },
    {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py","todo":"Add an \'edible\' property to the Frog class and initialize it in the constructor."}',
            name: "patch",
          },
          id: "call_dIXVNlzugvrPJvTF5G7n1YgK",
          index: 0,
          type: "function",
        },
      ],
    },
    {
      role: "diff",
      content: [
        {
          file_action: "edit",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 1,
          lines_add:
            "class Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n        self.edible = True",
          lines_remove:
            "class Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy",
        },
      ],
      tool_call_id: "call_dIXVNlzugvrPJvTF5G7n1YgK",
    },
    {
      role: "assistant",
      content:
        "The `Frog` class has been updated to include an `edible` property.",
      tool_calls: null,
    },
    // {
    //   role: "assistant",
    //   content:
    //     "The class `Frog` has been successfully renamed to `Bird` and all its references have been updated accordingly in the following files:\n\n- `frog.py`\n- `set_as_avatar.py`\n- `jump_to_conclusions.py`\n- `work_day.py`\n- `holiday.py`\n\nIs there anything else you need help with?",
    //   tool_calls: null,
    // },
  ],
  title: "rename the frog class to bird.\n",
  model: "gpt-4o",
  tool_use: "explore",
  createdAt: "2024-07-23T15:08:51.480Z",
  updatedAt: "2024-07-23T15:36:26.738Z",
};

export const TOOL_IMAGE_STUB: ChatMessages = [
  {
    role: "assistant",
    content: "",
    tool_calls: [
      {
        id: "a",
        function: {
          arguments:
            '{"command":"navigate_to https://www.wikipedia.org","tab":"new"}',
          name: "chrome",
        },
        type: "function",
        index: 0,
      },
    ],
  },
  {
    role: "tool",
    content: {
      tool_call_id: "a",
      content:
        "Opened new tab new\n\nChrome tab navigated to https://www.wikipedia.org/",
    },
  },
  {
    role: "assistant",
    content: "",
    tool_calls: [
      {
        id: "b",
        function: {
          arguments: '{"command":"screenshot","tab":"new"}',
          name: "chrome",
        },
        type: "function",
        index: 0,
      },
    ],
  },
  // {
  //   role: "tool",
  //   content: {
  //     tool_call_id: "b",
  //     content:
  //       "Using opened tab new\n\nMade a screenshot of https://www.wikipedia.org/",
  //   },
  // },
  {
    role: "tool",
    content: {
      tool_call_id: "b",
      content: [
        {
          m_type: "image/jpeg",
          m_content:
            "/9j/4AAQSkZJRgABAQAAAQABAAD/4gHYSUNDX1BST0ZJTEUAAQEAAAHIAAAAAAQwAABtbnRyUkdCIFhZWiAH4AABAAEAAAAAAABhY3NwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAA9tYAAQAAAADTLQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAlkZXNjAAAA8AAAACRyWFlaAAABFAAAABRnWFlaAAABKAAAABRiWFlaAAABPAAAABR3dHB0AAABUAAAABRyVFJDAAABZAAAAChnVFJDAAABZAAAAChiVFJDAAABZAAAAChjcHJ0AAABjAAAADxtbHVjAAAAAAAAAAEAAAAMZW5VUwAAAAgAAAAcAHMAUgBHAEJYWVogAAAAAAAAb6IAADj1AAADkFhZWiAAAAAAAABimQAAt4UAABjaWFlaIAAAAAAAACSgAAAPhAAAts9YWVogAAAAAAAA9tYAAQAAAADTLXBhcmEAAAAAAAQAAAACZmYAAPKnAAANWQAAE9AAAApbAAAAAAAAAABtbHVjAAAAAAAAAAEAAAAMZW5VUwAAACAAAAAcAEcAbwBvAGcAbABlACAASQBuAGMALgAgADIAMAAxADb/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAJABQADASIAAhEBAxEB/8QAHAABAAEFAQEAAAAAAAAAAAAAAAYCAwQFBwEI/8QAXRAAAQMDAgMEBgMIDQgIBAcBAAECAwQFEQYhEjFBBxNRYRQiMnGBkRWhsRYjM0JScsHRCDZWYnN0gpKUstLh8BckNDU3U5WzJUNUdZOiwvFEVWOkJjhXZaO00+L/xAAZAQEBAQEBAQAAAAAAAAAAAAAAAQMCBAX/xAA0EQEAAgIBAwIEAwYHAQEAAAAAAQIDESEEEjETQVFhcZEUIoEFFTIzobEjNEJSwdHh8PH/2gAMAwEAAhEDEQA/AO/gAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA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",
        },
      ],
    },
  },
];
