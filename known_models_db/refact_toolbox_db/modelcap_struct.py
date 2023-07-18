from pathlib import Path
from dataclasses import dataclass
from dataclasses_json import dataclass_json
from typing import Union, List


@dataclass_json
@dataclass
class ModelFunction:
    function_name: str
    metering: int
    label: str
    type: str
    selected_lines_min: int
    selected_lines_max: int
    third_party: Union[bool, List[bool]]
    supports_languages: str
    mini_html: str
    model: List[str]
    supports_highlight: bool = False
    supports_selection: bool = False
    supports_no_selection: bool = False
    model_fixed_intent: str = ""
    function_selection: str = ""
    function_hl_click: str = ""
    function_highlight: str = ""
    catch_all_selection: bool = False
    catch_all_hl: bool = False
    catch_question_mark: bool = False


def load_mini_html(name):
    file = Path(__file__).parent.joinpath('htmls', f'{name}.html')
    if not file.exists():
        raise FileNotFoundError(f'mini-html {file} does not exist')
    return file.read_text()

