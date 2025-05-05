---
title: Usage Based Pricing
sidebar_label: Usage Based Pricing
description: Learn about Refact.ai's new usage-based pricing model, how much you will be charged, and see a detailed per-model cost table.
---

Refact.ai uses a usage-based pricing system with coins. This page explains how coins work, how much you will be charged for different actions, and how to estimate your costs for each available model.

## How Coins Work

- **Coins are the unit of usage in Refact.ai.**
- **$1 = 1,000 coins.**
- You are only charged for the actual work performed by the AI Agent: simple tasks use fewer coins, complex ones use more.
- You choose the AI model for each task, and can stop tasks at any time to save coins.
- **Autocompletion is unlimited and free for all users.**

## Pricing Table (per 1M tokens)

<div id="pricing-toggle" style="margin-bottom: 1em;">
  <button id="show-coins" style="margin-right: 0.5em;">Show in Coins</button>
  <button id="show-dollars">Show in Dollars</button>
</div>

<table id="pricing-table">
  <thead>
    <tr>
      <th>Model</th>
      <th>Input Tokens</th>
      <th>Output Tokens</th>
      <th>Cache Read</th>
      <th>Cache Write</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>GPT-4o</td>
      <td data-coins="2500" data-dollars="2.50">$2.50</td>
      <td data-coins="10000" data-dollars="10.00">$10.00</td>
      <td data-coins="1250" data-dollars="1.25">$1.25</td>
      <td>-</td>
    </tr>
    <tr>
      <td>GPT-4o-mini</td>
      <td data-coins="150" data-dollars="0.15">$0.15</td>
      <td data-coins="600" data-dollars="0.60">$0.60</td>
      <td data-coins="75" data-dollars="0.075">$0.075</td>
      <td>-</td>
    </tr>
    <tr>
      <td>GPT-4.1</td>
      <td data-coins="2000" data-dollars="2.00">$2.00</td>
      <td data-coins="8000" data-dollars="8.00">$8.00</td>
      <td data-coins="500" data-dollars="0.50">$0.50</td>
      <td>-</td>
    </tr>
    <tr>
      <td>Claude 3.7 Sonnet</td>
      <td data-coins="3000" data-dollars="3.00">$3.00</td>
      <td data-coins="15000" data-dollars="15.00">$15.00</td>
      <td data-coins="300" data-dollars="0.30">$0.30</td>
      <td data-coins="3750" data-dollars="3.75">$3.75</td>
    </tr>
    <tr>
      <td>Claude 3.5 Sonnet</td>
      <td data-coins="3000" data-dollars="3.00">$3.00</td>
      <td data-coins="15000" data-dollars="15.00">$15.00</td>
      <td data-coins="300" data-dollars="0.30">$0.30</td>
      <td data-coins="3750" data-dollars="3.75">$3.75</td>
    </tr>
    <tr>
      <td>o3-mini</td>
      <td data-coins="1100" data-dollars="1.10">$1.10</td>
      <td data-coins="4400" data-dollars="4.40">$4.40</td>
      <td data-coins="550" data-dollars="0.55">$0.55</td>
      <td>-</td>
    </tr>
  </tbody>
</table>

<script>
const showCoinsBtn = document.getElementById('show-coins');
const showDollarsBtn = document.getElementById('show-dollars');
const table = document.getElementById('pricing-table');
function setTable(mode) {
  for (const row of table.tBodies[0].rows) {
    for (const cell of row.cells) {
      if (cell.dataset.coins && cell.dataset.dollars) {
        cell.textContent = mode === 'coins'
          ? cell.dataset.coins + ' coins'
          : '$' + Number(cell.dataset.dollars).toFixed(2);
      }
    }
  }
}
showCoinsBtn.onclick = () => setTable('coins');
showDollarsBtn.onclick = () => setTable('dollars');
</script>

> **Note:** 1,000 coins = $1. For example, generating 10,000 output tokens with GPT-4o would cost 10,000 coins (or $10).

> **Note:** 1,000 coins = $1. For example, generating 10,000 output tokens with GPT-4o would cost 150 coins ($0.15).

## Plans and Coin Grants

| Plan           | Monthly Coins | Details |
|----------------|--------------|---------|
| Free           | 5,000        | Complimentary $5 (5,000-coin) starter grant to explore the full capabilities of Refact.ai Agent. |
| Pro            | 10,000+      | $10/month = 10,000 coins. Pro users can increase their monthly limits by 2×, 3×, 4×, or 5× (e.g., $20 = 20,000 coins; $30 = 30,000 coins, etc.). You will receive exactly 2, 3, 4, or 5 times the coins for the corresponding plan multiplier. Unused coins roll over to the next month. One-time top-ups are available from $5. |

## What’s Included in Each Plan

| FREE           | PRO |
|----------------|-----|
| $0 / month     | $10 / month |
| 5,000 coins to use AI Agent & Chat | 10,000 coins renewed every month; unused coins roll over |
| In-IDE chat aware of your codebase context | Top up from $5 in your account ($1 = 1,000 coins) |
| Claude 3.7, GPT 4.1, 4o, Gemini 2.5 pro, and more | Subscribe to a 2x-5x Pro plan to top up automatically |
| Unlimited fast auto-completion | |
| Codebase-aware vector database (RAG) | |
| Self-hosting option available | |
| Discord support | |

## Bring Your Own Key (BYOK)

If you prefer to use your own API key (for OpenAI, Anthropic, or local models), you can connect it to Refact.ai. When using BYOK, requests are billed by your provider and do not consume Refact.ai coins.

**No commission:** For now, Refact.ai does not take any commission or markup on API usage. You pay only for the actual API cost of the model you use.

For more information on how to use Bring Your Own Key (BYOK), see the [BYOK documentation](https://github.com/smallcloudai/refact/blob/main/docs/byok.md) in the repository.
