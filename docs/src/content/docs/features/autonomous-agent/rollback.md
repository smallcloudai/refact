---
title: Agent Rollback
description: Learn how to use Refact.ai Agent's rollback functionality to revert repository changes
---

The Refact.ai Agent provides a powerful rollback feature that allows you to revert your repository to the state it was in at any specific point in your chat conversation. This feature is particularly useful when experimenting with different solutions or when you need to undo a series of changes.

## Overview

The rollback functionality enables you to:
- Return to any previous state in your development session
- Review changes before applying the rollback
- Maintain a clear history of your development process

## Enabling Rollback

The rollback feature is enabled by default in Agent chat mode. However, you should verify that the toggle is properly set:

<div class="video-frame">
  <video controls width="100%">
    <source src="/videos/enable_rollback.mp4" type="video/mp4">
    Your browser does not support the video tag.
  </video>
</div>

## Using Rollback

To rollback to a specific point in your conversation:

1. Navigate to the message you want to rollback to in the chat history
2. Look for the rollback icon next to the message
3. Click the icon to initiate the rollback process
4. Review the summary of changes that will occur
5. Confirm the rollback if you agree with the changes

Here's how to use the rollback feature:

<div class="video-frame">
  <video controls width="100%">
    <source src="/videos/use_rollback.mp4" type="video/mp4">
    Your browser does not support the video tag.
  </video>
</div>

## Important Considerations

:::caution[Warning]
Any changes made to your repository after the selected message point will be removed during rollback. This includes:
- Changes made by the Agent
- Manual changes you've made
- Any file modifications or additions
:::

## Next Steps

- Learn about [Agent Tools](../tools) to better understand how changes are made
- Explore [Agent Integrations](../integrations) for enhanced development workflow
- Check out the [FAQ](/faq) for common questions about Agent features