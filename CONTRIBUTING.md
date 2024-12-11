
# 🌟 Contribute to Refact.ai

Welcome to the Refact.ai project! We’re excited to have you join our community. Whether you’re a first-time contributor or a seasoned developer, here are some impactful ways to get involved.

---

## 📚 Table of Contents

- [❤️ Ways to Contribute](#-ways-to-contribute)
  - [🐛 Report Bugs](#-report-bugs)
  - [✨ Suggest Features](#-suggest-features)
  - [📖 Improving Documentation](#-improving-documentation)
    - [Running the Documentation Server Locally](#running-the-documentation-server-locally)
  - [Contributing To Code](#contributing-to-code)
    - [Pre-Requisites](#pre-requistes)
    - [Installation](#installation)
    - [Fork the Repository](#fork-the-repository)
    - [How to Create Good PR](#how-to-create-good-pr)
    - [Install Linguist](#install-linguist)
   

---
     

# 🚀 Ways to Contribute

Our GitHub project board is a treasure trove of ideas on how you can contribute. These are just starting points—feel free to explore and propose your own initiatives!

*github project board soon*

---

## 🐛 Report Bugs

Encountered an issue? Help us improve Refact.ai by reporting bugs in issue section, make sure you label the issue with correct tag [here](https://github.com/smallcloudai/refact/issues)! 

A comprehensive bug report includes:

- **Summary**: A brief overview of the issue.
- **Steps to Reproduce**: A detailed walkthrough of how to replicate the problem.
- **Expected vs. Actual Behavior**: Clarify what you expected to happen and what actually occurred.
- **Visuals**: Attach screenshots or videos to illustrate the issue.

---

## ✨ Suggest Features

We’re constantly evolving, and your ideas matter, make sure you label the issue with correct tag! To propose a new feature:

1. **Research Existing Proposals**: Check if your idea has already been suggested.
2. **Open a New Issue**: If it’s unique, please create a new issue.
3. **Provide Details**: Explain your feature idea and its potential impact on user experience.
4. **Engage with the Community**: Join us on the Refact.ai Discord.

---

## 📖 Improving Documentation

Help us make Refact.ai more accessible by contributing to our documentation, make sure you label the issue with correct tag! Create issues [here](https://github.com/smallcloudai/web_docs_refact_ai/issues).

### Running the Documentation Server Locally
Refer to this Repo : https://github.com/smallcloudai/web_docs_refact_ai


---

## Contributing To Code

### Pre-Requistes

Ensure you have the necessary tools and dependencies installed to kick off your development journey smoothly. Check our setup guide for detailed instructions.

### Installation

Clone this repo and install it for development:

```commandline
git clone https://github.com/smallcloudai/refact
pip install -e refact/
```

To run the whole server, use:

```commandline
python -m self_hosting_machinery.watchdog.docker_watchdog
```

For debugging, it's better to run HTTP server and inference processes separately, for example in
separate terminals.

```commandline
python -m self_hosting_machinery.webgui.webgui
DEBUG=1 python -m self_hosting_machinery.inference.inference_worker --model wizardlm/7b
```

That should be enough to get started!

### Fork the Repository

If you plan to make changes, you need your own fork of the project -- clone that instead of
the main repo. Once you have your changes ready, commit them and push them to your fork. After
that you should be able to create a pull request for the main repository.

---

### How to Create Good PR

Check out this here : https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/getting-started/helping-others-review-your-changes


### Install Linguist

For fine tuning, files go through a pre filter. Follow instructions in
https://github.com/smallcloudai/linguist
to install it.

If you don't plan to debug fine tuning, you can skip this step.


