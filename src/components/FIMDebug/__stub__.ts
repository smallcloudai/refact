import type { FimDebugData } from "../../events";

export const STUB: FimDebugData = {
  choices: [
    {
      code_completion:
        '"refact_scratchpads_no_gpu",\n        "stream_results",',
      finish_reason: "stop",
      index: 0,
    },
  ],
  context: {
    attached_files: [
      {
        file_content:
          '...19 lines\n__all__ = ["TabUploadRouter", "download_file_from_url", "UploadViaURL"]\n...2 lines\nasync def download_file_from_url(url: str, download_dir: str, force_filename: Optional[str] = None) -> str:\n...39 lines\nclass UploadViaURL(BaseModel):\n    url: str\n...2 lines\nclass CloneRepo(BaseModel):\n    url: str\n    branch: Optional[str] = None\n...2 lines\nclass TabSingleFileConfig(BaseModel):\n    which_set: str = Query(default=Required, regex="auto|train|test")\n    to_db: bool = Query(default=False)\n...2 lines\nclass TabFilesConfig(BaseModel):\n    uploaded_files: Dict[str, TabSingleFileConfig]\n...2 lines\nclass FileTypesSetup(BaseModel):\n    filetypes_finetune: Dict[str, bool] = Query(default={})\n    filetypes_db: Dict[str, bool] = Query(default={})\n    force_include: str = Query(default="")\n    force_exclude: str = Query(default="")\n...2 lines\nclass TabFilesDeleteEntry(BaseModel):\n    delete_this: str = Query(default=Required, regex=r\'^(?!.*\\/)(?!.*\\.\\.)[\\s\\S]+$\')\n...2 lines\nclass ProjectNameOnly(BaseModel):\n    pname: str = Query(default=Required, regex=r\'^[A-Za-z0-9_\\-\\.]{1,30}$\')\n...2 lines\nclass TabUploadRouter(APIRouter):\n\n    def __init__(self, *args, **kwargs):\n...13 lines\n\n    async def _tab_project_new(self, project: ProjectNameOnly):\n...7 lines\n\n    async def _tab_project_list(self):\n...6 lines\n\n    async def _tab_project_delete(self, project: ProjectNameOnly):\n...8 lines\n\n    async def _tab_files_get(self, pname):\n...\n',
        file_name:
          "/Users/valaises/PycharmProjects/refact-self-hosting/refact_webgui/webgui/tab_upload.py",
        line1: 20,
        line2: 385,
      },
      {
        file_content:
          '...37 lines\nclass WebGUI(FastAPI):\n\n    def __init__(self,\n                 model_assigner: ModelAssigner,\n                 database: RefactDatabase,\n                 stats_service: StatisticsService,\n                 session: RefactSession,\n                 *args, **kwargs):\n...19 lines\n\n    def _setup_middlewares(self):\n...15 lines\n        )\n\n    @staticmethod\n    def _routers_list(\n            id2ticket: Dict[str, Ticket],\n            inference_queue: InferenceQueue,\n            model_assigner: ModelAssigner,\n            stats_service: StatisticsService,\n            session: RefactSession):\n...31 lines\n        ]\n\n    async def _startup_event(self):\n...15 lines\ndef setup_logger():\n    # Suppress messages like this:\n    # WEBUI 127.0.0.1:55610 - "POST /infengine-v1/completions-wait-batch HTTP/1.1" 200\n    # WEBUI 127.0.0.1:41574 - "POST /infengine-v1/completion-upload-results\n...\n',
        file_name:
          "/Users/valaises/PycharmProjects/refact-self-hosting/refact_webgui/webgui/webgui.py",
        line1: 38,
        line2: 191,
      },
      {
        file_content:
          '...14 lines\nlogger = logging.getLogger("INFSERVER")\n...2 lines\nurls_to_try = [\n    "http://127.0.0.1:8008/infengine-v1/",\n]\n...2 lines\ndef override_urls(*urls):\n...4 lines\nurls_switch_n = 0\nurls_switch_ts = time.time()\n...2 lines\ndef infserver_session() -> requests.Session:\n...8 lines\ndef url_get_the_best():\n...6 lines\ndef url_complain_doesnt_work():\n...5 lines\ndef model_guid_allowed_characters(name):\n...3 lines\ndef validate_description_dict(\n    infeng_instance_guid: str,\n    account: str,\n    model: str,\n    B: int,\n    max_thinking_time: int,\n):\n...9 lines\n    }\n...2 lines\ndef completions_wait_batch(req_session: requests.Session, my_desc, verbose=False):\n...38 lines\ndef head_and_tail(base: str, modified: str):\n...20 lines\ndef test_head_and_tail():\n...7 lines\nDEBUG_UPLOAD_NOT_SEPARATE_PROCESS = False\n...2 lines\nclass UploadProxy:\n    def __init__(\n            self,\n            upload_q: Optional[multiprocessing.Queue] = None,\n            cancelled_q: Optional[multiprocessing.Queue] = None,\n    ):\n...4 lines\n\n...\n',
        file_name:
          "/Users/valaises/PycharmProjects/refact-self-hosting/refact_scratchpads_no_gpu/stream_results.py",
        line1: 15,
        line2: 363,
      },
      {
        file_content:
          '...16 lines\nFIRST_RUN_CMDLINE = [sys.executable, "-m", "self_hosting_machinery.scripts.first_run"]\n...2 lines\ndef replace_variable_names_from_env(s):\n...7 lines\nlog_prevdate = ""\n\ndef log(*args):\n...18 lines\ncompile_successful = set()\ncompile_unsuccessful = set()\ncompiling_now = ""\n...2 lines\ndef cfg_to_cmdline(cfg):\n...7 lines\ndef cfg_to_compile_key(cfg):\n...6 lines\nclass TrackedJob:\n    def __init__(self, cfg, cfg_filename: Path):\n...12 lines\n\n    def set_status(self, newstatus):\n...18 lines\n\n    def _start(self):\n...42 lines\n\n    def _poll_logs(self) -> bool:\n...\n',
        file_name:
          "/Users/valaises/PycharmProjects/refact-self-hosting/self_hosting_machinery/watchdog/docker_watchdog.py",
        line1: 17,
        line2: 462,
      },
    ],
    was_looking_for: [
      {
        from: "cursor_usages",
        symbol: "initialization_for_scripts",
      },
      {
        from: "cursor_usages",
        symbol: "start_rust",
      },
      {
        from: "cursor_usages",
        symbol: "start",
      },
      {
        from: "cursor_usages",
        symbol: "__name__",
      },
      {
        from: "cursor_usages",
        symbol: "start",
      },
    ],
  },
  created: 1712158050.521,
  model: "starcoder2/7b/base",
  snippet_telemetry_id: 100,
};
