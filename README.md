<a name="readme-top"></a>

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://docs.refact.ai/_astro/logo-dark.CCzD55EA.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://docs.refact.ai/_astro/logo-light.CblxRz3x.svg">
    <!-- Fallback if neither preference is set -->
    <img alt="Refact.ai logo" src="https://docs.refact.ai/_astro/logo-dark.CCzD55EA.svg" width="200">
  </picture>
  <h1 align="center">Refact - Open Sourced AI Software Development Agent</h1>
</div>

<div align="center">
  <a href="https://github.com/smallcloudai/refact/stargazers"><img src="https://img.shields.io/github/stars/smallcloudai/refact?style=for-the-badge&color=blue" alt="Stargazers"></a>
  <a href="https://discord.gg/Kts7CYg99R"><img src="https://img.shields.io/badge/Discord-Join%20Us-purple?logo=discord&logoColor=white&style=for-the-badge" alt="Join our Discord community"></a>
  <a href="https://docs.refact.ai"><img src="https://img.shields.io/badge/documentation-blue?logo=googledocs&logoColor=FFE165&style=for-the-badge" alt="Check out the documentation"></a>
  
</div>



Refact Agent is a free, open-source AI Agent that handles engineering tasks end-to-end. It deeply understands your codebases and integrates with your tools, databases, and browsers to automate complex, multi-step tasks.


## 🚀 Seamless Integration with Your Workflow  

Refact Agent works effortlessly with the tools and databases you already use:  


- **📁 Version Control:** GitHub, GitLab  
- **🗄️ Databases:** PostgreSQL, MySQL  
- **🛠️ Debugging:** Pdb  
- **🐳 Containerization:** Docker  

### ⚡ Why Choose Refact Agent?  

- ✅ **Deploy On-Premise:** Maintain **100% control** over your codebase.  
- 🧠 **Access State-of-the-Art Models:** Supports Claude 3.5 Sonnet, GPT-4o, o3-mini, and more.  
- 🔑 **Bring Your Own Key (BYOK):** Use your own API keys for external LLMs.  
- 💬 **Integrated IDE Chat:** Stay in your workflow, no need to switch between tools!  
- ⚡ **Free, Unlimited, Context-Aware Auto-Completion:** Code faster with smart AI suggestions.  
- 🛠️ **Supports 25+ Programming Languages:** Python, JavaScript, Java, Rust, TypeScript, PHP, C++, C#, Go, and many more!  


📜 [View Full List of Supported Models](https://docs.refact.ai/supported-models/) 

> 📢  **Using AI for work? Let’s bring it to your company!** 
> 
> [Fill out this form](https://refact.ai/contact/?utm_source=github&utm_medium=readme&utm_campaign=enterprise) — Our AI Agent will be tailored to your company’s data, learning from feedback, and helping organize knowledge for **better collaboration** with your team.


## 📚 Table of Contents

- 🚀 [Core Features and Functionality](#-core-features-and-functionality)
- 🤖 [Which Tasks Can Refact Help You With?](#-which-tasks-can-refact-help-you-with)
- ⚙️ [QuickStart](#%EF%B8%8F-quickstart)
- 🐳 [Running Refact Self-Hosted in a Docker Container](#-running-refact-self-hosted-in-a-docker-container)
- 🔌 [Getting Started with Plugins](#-getting-started-with-plugins)
- 📖 [Documentation](#-documentation)
- 🥇 [Contribution](#-contribution)
- 🎉 [Join the Community](#-join-the-community)

## 🚀 Core Features and Functionality

 ✅ **Unlimited accurate auto-completion** with context awareness – Powered by Qwen2.5-Coder-1.5B, utilizing Retrieval-Augmented Generation (RAG).  

![auto-completion](https://lh7-rt.googleusercontent.com/docsz/AD_4nXfClhl11Ul0YQjDTZJvrfhsj3bqK_VIz6bFfbTRc62dsMOz4LK4u72i9-gLTQDIgm0yChmFe57hvUxSoI2fQ5DSntna7_Ch0qbGx5zcB-othfwKnoYkbt3M3YgGFlrqFszuDEBhUw?key=zllGjEBckkx13bRZ6JIqX6qr)

 ✅ **Integrated in-IDE Chat** – AI deeply understands your code and provides relevant, intelligent answers.  

 ✅ **Integrated with Tools** – Works with GitHub, GitLab, PostgreSQL, MySQL, Pdb, Docker, and shell commands.  

![integrations](https://lh7-rt.googleusercontent.com/docsz/AD_4nXc4DWYXF73AgPWAaFFGLTqEprWwA0im8R_A1QMo4QW4pTnSi1MCoP9L8udMZb5FPyN-CdgefaxJFGpX2ndn5nkjGBF2b_hZBNHogM7IM6SPvUIvUd9iE1lYIq7q-TB2qKzSGLk00A?key=zllGjEBckkx13bRZ6JIqX6qr)

 ✅ **State-of-the-Art Models** – Supports Claude 3.5 Sonnet, GPT-4o, o3-mini, and more.  

 ✅ **Bring Your Own Key (BYOK)** – Use your own API keys for external LLMs.  

![BYOK](https://lh7-rt.googleusercontent.com/docsz/AD_4nXe1UDsuaER6WMxAnKEwz15T3OPslkpSo2vNGMGaNoEiZOJvAptY8yEvND_rI23q_5Sof1DceexyrW5x6oUwcpVr5KQvWUByrN_TnLGVY2HG_0sg8uWnRb14jKAes2MBDPM37EQO?key=zllGjEBckkx13bRZ6JIqX6qr)


## 🤖 Which Tasks Can Refact Help You With?

- 🏗 **Generate code** from natural language prompts (even with typos).  

- 🔄 **Refactor code** for better quality and readability.  

- 📖 **Explain code** to quickly understand unfamiliar code.  

- 🐞 **Debug code** to detect and fix errors faster.  

- 🧪 **Generate unit tests** for reliable code.  

- 📌 **Code Review** with AI-assisted suggestions.  

- 📜 **Create Documentation** to keep knowledge up to date. 
 
- 🏷 **Generate Docstrings** for structured documentation.  



## ⚙️ QuickStart

You can install the Refact repository without Docker:
```bash
pip install .
```

For GPU with CUDA capability >= 8.0 and flash-attention v2 support:
```bash
FLASH_ATTENTION_FORCE_BUILD=TRUE MAX_JOBS=4 INSTALL_OPTIONAL=TRUE pip install .
```



## 🐳 Running Refact Self-Hosted in a Docker Container

The easiest way to run the self-hosted server is using a pre-built Docker image.  
See `CONTRIBUTING.md` for installation without a Docker container.


### 🔌 Getting Started with Plugins

1. **Download Refact** for VS Code or JetBrains.  
2. **Set up a custom inference URL:**  
   ```
   http://127.0.0.1:8008
   ```
3. **Configure the plugin settings:**  
   - **JetBrains:** Settings > Tools > Refact.ai > Advanced > Inference URL  
   - **VSCode:** Extensions > Refact.ai Assistant > Settings > Address URL  



## 📖 Documentation

For detailed guidance and best practices, check out our [documentation.](https://docs.refact.ai/)


## 🥇 Contribution

Want to contribute to our project? We're always open to new ideas and features!  
- **Check out GitHub Issues** – See what we're working on or suggest your own ideas.  
- **Read our Contributing Guide** – Check out `Contributing.md` to get started.  

Your contributions help shape the future of Refact Agent! 🚀



### 🎉 Join the Community

We're all about open-source and empowering developers with AI tools. Our vision is to build the future of programming. Join us and be part of the journey!

📢 **[Join our Discord server](https://refact.ai/community/)** – A community-run space for discussion, questions, and feedback.



**Made with ❤️ by developers who automate the boring, so you can focus on building the future.**


