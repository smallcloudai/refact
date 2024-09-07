import asyncio
from prompt_toolkit import PromptSession
from prompt_toolkit.patch_stdout import patch_stdout
from prompt_toolkit.shortcuts import print_formatted_text
from prompt_toolkit.formatted_text import FormattedText
from prompt_toolkit.styles import Style

progress_data = {
    'Indexing files': 0,
    'Processing data': 0,
    'Finalizing': 0
}

async def update_progress():
    """Simulate background progress updates."""
    while True:
        for task in progress_data:
            if progress_data[task] < 100:
                progress_data[task] += 10
                await asyncio.sleep(1)
        await asyncio.sleep(1)

style = Style.from_dict({
    'green': 'ansigreen',
    'yellow': 'ansiyellow',
})

def display_progress():
    """Display the progress above the prompt."""
    lines = []
    for task, progress in progress_data.items():
        color = 'green' if progress == 100 else 'yellow'
        lines.append((color, f"{task}: {progress}%\n"))
    return FormattedText(lines)


async def chat_main():
    session = PromptSession()
    asyncio.create_task(update_progress())

    with patch_stdout():
        while True:
            print_formatted_text(display_progress(), style=style)
            try:
                user_input = await session.prompt_async('> ')
                if user_input.lower() in ('exit', 'quit'):
                    break
                print(f'You entered: {user_input}')
            except EOFError:
                print("\nclean exit")
                break


def cmdline_main():
    asyncio.run(chat_main())


if __name__ in '__main__':
    asyncio.run(chat_main())
