import base64

import requests
import json
import pathlib
from termcolor import colored


BASE_DIR = pathlib.Path(__file__).parent
DATA_DIR = BASE_DIR / "test13_data"

IMAGE_200 = DATA_DIR / "200.jpg"
IMAGE_530 = DATA_DIR / "530.jpg"


def get_base64_image(image_path):
    def encode_image(image_path):
        with open(image_path, "rb") as image_file:
            return base64.b64encode(image_file.read()).decode('utf-8')

    return encode_image(image_path)


def chat_request(msgs, max_tokens: int = 200):
    url = "http://localhost:8001/v1/chat"
    payload = {
        "model": "gpt-4o",
        "messages": msgs,
        "stream": False,
        "max_tokens": max_tokens,
    }
    resp = requests.post(url, data=json.dumps(payload))
    j = resp.json()
    return j["choices"][0]["message"]["content"]


def test_format():
    messages = [
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "marco"
                },
            ]
        }
    ]
    m = chat_request(messages, 5)
    assert m.lower().startswith("polo"), m
    print(colored("test_format PASSED", "green"))


def test_image_sending():
    image200 = get_base64_image(IMAGE_200)
    messages = [
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "print number you see on the image, respond only numbers"
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": f"data:image/jpeg;base64,{image200}"
                    }
                }
            ]
        }
    ]
    m = chat_request(messages,5)
    assert "200" in m, m
    print(colored("test_image_sending PASSED", "green"))


def test_multiple_images_sending():
    image200 = get_base64_image(IMAGE_200)
    image530 = get_base64_image(IMAGE_530)
    messages = [
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "print numbers you see on images. respond only numbers"
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": f"data:image/jpeg;base64,{image200}"
                    }
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": f"data:image/jpeg;base64,{image530}"
                    }
                }
            ]
        }
    ]
    m = chat_request(messages, 30)
    assert "200" in m, m
    assert "530" in m, m
    print(colored("test_multiple_images_sending PASSED", "green"))


if __name__ == "__main__":
    test_format()
    test_image_sending()
    test_multiple_images_sending()
