---
title: Agent Overview
description: Comprehensive overview of Refact.ai Agent capabilities and features
---

Refact.ai Agent is now available, bringing autonomous capabilities to your development workflow. The agent can independently gather context, create and edit documents, execute shell commands, and much more.

## Real-time Collaboration
A standout feature of Refact.ai Agent is its real-time awareness of your actions. The agent automatically stays in sync with your codebase, eliminating the need to manually provide context about recent changes.

Watch how our Agent can read and understand recently created files:

<div class="video-frame">
  <video controls width="100%">
    <source src="/videos/Access_Context.mp4" type="video/mp4">
    Your browser does not support the video tag.
  </video>
</div>

## Autonomous Patching
Refact.ai Agent can autonomously create and edit files. The process works in two steps:

1. The Agent provides the code changes it wants to make
2. You approve the "Patches" before they're applied

Watch how Refact.ai Agent is rewrite file from python to php:
<div class="video-frame">
  <video controls width="100%">
    <source src="/videos/Patch.mp4" type="video/mp4">
    Your browser does not support the video tag.
  </video>
</div>

You can streamline the process by enabling automatic patch approval.
Toggle the "Allow Patches" option to let the Agent make changes without requiring permission each time:

<div class="video-frame">
  <video controls width="100%">
    <source src="/videos/Auto_Apply.mp4" type="video/mp4">
    Your browser does not support the video tag.
  </video>
</div>

## Shell Command Execution

<div style="display: grid; grid-template-columns: 1fr 2fr; gap: 1rem; align-items: center;">
  <div>
    The Agent can execute terminal commands on your behalf
  </div>
  <div class="video-frame">
    <video controls width="100%">
      <source src="/videos/Terminal_Commands.mp4" type="video/mp4">
      Your browser does not support the video tag.
    </video>
  </div>
</div>

See an example of the Agent creating a virtual environment and installing numpy.


For setup instructions, visit the [Agent Integrations](/features/autonomous-agent/integrations/) page.
