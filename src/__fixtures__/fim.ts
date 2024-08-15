import type { FimDebugData } from "../services/refact";

export const STUB: FimDebugData = {
  choices: [
    {
      code_completion:
        '    """\n    This is a comment for AmericanCommonToad class, inside the class\n    """',
      finish_reason: "stop",
      index: 0,
    },
  ],
  context: {
    attached_files: [
      {
        file_content:
          "...\nclass LSPCall:\n    def __init__(\n            self,\n            connect_options: LSPConnectOptions\n    ):\n        self._connect_options = connect_options\n\n    def __enter__(self):\n        self.connect()\n        return self\n\n    def __exit__(self, exc_type, exc_val, exc_tb):\n        self.shutdown()\n\n    def load_document(\n            self,\n            file_name: str,\n            text: str,\n            version: int = 1,\n            language: str = 'python'\n    ):\n        if language == 'python':\n            languageId = pylspclient.lsp_structs.LANGUAGE_IDENTIFIER.PYTHON  # noqa;\n        else:\n            raise NotImplemented(f\"language {language} is not implemented for LSPCall.load_document\")\n        uri = os.path.join(self._connect_options.root_uri, file_name)\n        self._lsp_client.didOpen(pylspclient.lsp_structs.TextDocumentItem(uri, languageId, version, text=text))\n\n    def get_completions(\n            self,\n            file_name,\n            pos: Tuple[int, int],\n            params: Optional[Dict] = None,\n            multiline: bool = False\n    ):\n        if not params:\n            params = {\n                \"max_new_tokens\": 20,\n                \"temperature\": 0.1\n            }\n\n        uri = os.path.join(self._connect_options.root_uri, file_name)\n        cc = self._lsp_client.lsp_endpoint.call_method(\n            \"refact/getCompletions\",\n            textDocument=pylspclient.lsp_structs.TextDocumentIdentifier(uri),\n            position=pylspclient.lsp_structs.Position(pos[0], pos[1]),\n            parameters=params,\n            multiline=multiline,\n        )\n        return cc\n\n    def connect(self):\n        self._connect2lsp(self._connect_options)\n\n    def shutdown(self):\n        print(colored('LSPCall is shutting down...', 'magenta'))\n        try:\n            self._lsp_client.shutdown()\n            self._lsp_endpoint.join()\n        except Exception:\n            pass\n\n    def _connect2lsp(self, connect_options: LSPConnectOptions):\n        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)\n        s.connect(\n            (connect_options.addr, connect_options.port)\n        )\n        pipe_in, pipe_out = s.makefile(\"wb\", buffering=0), s.makefile(\"rb\", buffering=0)\n        json_rpc_endpoint = pylspclient.JsonRpcEndpoint(pipe_in, pipe_out)\n        self._lsp_endpoint = pylspclient.LspEndpoint(json_rpc_endpoint)\n        self._lsp_client = pylspclient.LspClient(self._lsp_endpoint)\n        capabilities = {}\n        workspace_folders = [{'name': 'workspace', 'uri': connect_options.root_uri}]\n...\n        )\n",
        file_name:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/lsp_connect.py",
        line1: 18,
        line2: 99,
      },
      {
        file_content:
          "...\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n",
        file_name:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/frog.py",
        line1: 4,
        line2: 27,
      },
      {
        file_content: "...\n    f.jump(W, H)\n",
        file_name:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/work_day.py",
        line1: 11,
        line2: 11,
      },
    ],
    bucket_declarations: [
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/lsp_connect.py",
        line1: 20,
        line2: 24,
        name: "__init__",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/frog.py",
        line1: 22,
        line2: 27,
        name: "jump",
      },
    ],
    bucket_high_overlap: [
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/frog.py",
        line1: 22,
        line2: 27,
        name: "jump",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/frog.py",
        line1: 6,
        line2: 10,
        name: "__init__",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/frog.py",
        line1: 12,
        line2: 20,
        name: "bounce_off_banks",
      },
    ],
    bucket_usage_of_same_stuff: [
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/work_day.py",
        line1: 12,
        line2: 12,
        name: "jump",
      },
    ],
    cursor_symbols: [
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 28,
        line2: 28,
        name: "AmericanCommonToad",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 31,
        line2: 31,
        name: "__name__",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 26,
        line2: 26,
        name: "self",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 26,
        line2: 26,
        name: "name",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 32,
        line2: 32,
        name: "EuropeanCommonToad",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 32,
        line2: 32,
        name: "toad",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 25,
        line2: 25,
        name: "x",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 25,
        line2: 25,
        name: "y",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 25,
        line2: 25,
        name: "vx",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 25,
        line2: 25,
        name: "vy",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 25,
        line2: 25,
        name: "super",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 25,
        line2: 25,
        name: "__init__",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 33,
        line2: 33,
        name: "W",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 33,
        line2: 33,
        name: "H",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 33,
        line2: 33,
        name: "jump",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 34,
        line2: 34,
        name: "print",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 13,
        line2: 16,
        name: "Toad",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 7,
        line2: 7,
        name: "X",
      },
      {
        file_path:
          "/Users/marc/Projects/smallcloudai/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
        line1: 7,
        line2: 7,
        name: "Y",
      },
    ],
    fim_ms: 10,
    n_ctx: 2048,
    rag_ms: 24,
    rag_tokens_limit: 1024,
  },
  created: 1713530045.604,
  model: "starcoder2/7b/vllm",
  snippet_telemetry_id: 101,
};
