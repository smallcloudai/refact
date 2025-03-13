---
title: Chrome Tool
description: Configure and use the Chrome Tool for web interactions
---

The Chrome Tool enables the Refact.ai Agent to interact with browsers like Chrome, Chromium, and Edge. This integration allows the agent to open links and take screenshots to extract additional context for your tasks.

## Functionality

### Enable Toggle
- Allows you to activate or deactivate the Chrome Tool
- When toggled on, the tool is ready for use; toggling off disables its functionality

### Delete Button
- Permanently removes the Chrome Tool from your list of integrations
- Use this if you no longer need the tool

### Test
- Opens a chat interface and tests whether the Chrome Tool is correctly configured
- If successful, it confirms that the tool is ready to use

### Help Me Install Chrome for Testing
- Launches a chat to guide you through configuring the tool automatically by identifying the browser path
- ⚠️ Note: This process doesn't always work flawlessly — think of it as the model taking a stab at finding the path on your behalf (sometimes it hits the mark, sometimes it doesn't)

## Configuration Instructions

### Find the Browser Path
The process for identifying the browser path depends on your operating system. Here are typical paths:

- Windows: `C:\Program Files\Google\Chrome\Application\chrome.exe`
- MacOS: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`
- Linux: `/usr/bin/google-chrome` or `/usr/bin/chromium`

### Inserting the Path
1. Copy the full path to your browser executable
2. Paste it into the Chrome Path field
3. Click Apply to save the configuration

## Advanced Configuration Options

### Idle Browser Timeout
- Sets the time (in seconds) the browser can remain idle before being closed automatically
- Default: 300 seconds

### Headless Mode
- When enabled, Chrome runs in "headless" mode (no visible browser window)
- Default: true (enabled)

### Window Dimensions and Scale Factors
Configure the size and scaling for different device views:

#### Desktop
- Window Width: 1280 (default)
- Window Height: 800 (default)
- Scale factor: 1 (default)

#### Mobile
- Window Width: 667 (default)
- Window Height: 375 (default)
- Scale Factor: 1 (default)

#### Tablet
- Window Width: 768 (default)
- Window Height: 1024 (default)
- Scale Factor: 1 (default)

## Usage Examples

### Opening Web Links

<div style="display: grid; grid-template-columns: 2fr 3fr; gap: 1rem; align-items: center;">
  <div>
    You can ask the Agent to:
    - Open any URL and take a screenshot
    - Navigate to specific elements on a page
    - Interact with web content
  </div>
  <div class="video-frame">
    <video controls width="100%">
      <source src="/videos/Open_Link_Screenshot.mp4" type="video/mp4">
      Your browser does not support the video tag.
    </video>
  </div>
</div>

### Local File Access

<div style="display: grid; grid-template-columns: 2fr 3fr; gap: 1rem; align-items: center;">
  <div>
    The Agent can also:
    - Open locally hosted websites
    - Take screenshots
  </div>
  <div class="video-frame">
    <video controls width="100%">
      <source src="/videos/Local_Link_Screenshot.mp4" type="video/mp4">
      Your browser does not support the video tag.
    </video>
  </div>
</div>