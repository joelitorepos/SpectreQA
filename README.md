# 🔍 SpectreQA

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Built with Bun](https://img.shields.io/badge/Built%20with-Bun-fbf0df?style=flat&logo=bun&logoColor=black)](https://bun.sh/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-v19-61dafb?style=flat&logo=react&logoColor=black)](https://react.dev/)
[![Chrome Web Store](https://img.shields.io/badge/Chrome_Web_Store-SpectreQA-4285F4?style=flat&logo=googlechrome&logoColor=white)](https://chromewebstore.google.com/detail/spectreqa/eonjhhhdhlmlhbcmccenadolnkpingpb)

English | [Leer en Español](README.es.md)

**SpectreQA** is an AI-powered ecosystem for automated end-to-end (E2E) web testing and interaction. Built on a hybrid architecture combining a lightweight desktop application (Tauri + React), a browser extension, and LLM integrations (cloud or local), SpectreQA audits and validates complex interactive user flows in web applications using natural language prompts.

---

## 🛠️ Prerequisites & Installation

To use SpectreQA, you will need:

1. **Desktop Application:** Download the compiled binary or build it directly from this repository.
2. **Browser Extension:** Available directly on the **Chrome Web Store**:  
   🔗 [Install SpectreQA Extension](https://chromewebstore.google.com/detail/spectreqa/eonjhhhdhlmlhbcmccenadolnkpingpb)
3. *(Optional)* **Ollama:** If you prefer running 100% local language models on your machine for private testing, install [Ollama](https://ollama.com/) and ensure it is running in the background.

---

## 🚀 Development & Build Commands

This project uses **Bun** as its runtime and package manager.

### Development Mode (Frontend in Browser)
```bash
bun run dev

```

### Run Desktop App (Development Mode)

Launches the native Tauri window with Hot Reloading enabled:

```bash
bun run tauri dev

```

### Production Build

Generates optimized distribution binaries:

```bash
bun run tauri build

```

* **Linux (Debian/Ubuntu/Mint):** Generates `.deb` packages and binary images in:
```bash
cd src-tauri/target/release/bundle/deb/
sudo apt install ./SpectreQA_0.1.0_amd64.deb

```


* **Windows:** Generates an executable installer (`.exe` / `.msi`) in:
```text
src-tauri\target\release\bundle\nsis\

```



---

## 💻 Step-by-Step Usage Guide

### 1. Registering a Project

Upon launching SpectreQA, configure the environment for the web application you want to test:

* **Public / Production Environment:** Enter a Project Name and its public URL (e.g., `https://my-app.com`).
* **Local Environment:** Enter a Project Name, Host/Port (e.g., `http://localhost:3000`), the absolute path to your project's root folder, and the start command (e.g., `npm run dev`).

### 2. Defining the Test Objective (*Prompt*)

* Hover over the project card in SpectreQA and click the configuration/file icon.
* Write a detailed **prompt** specifying what the AI should validate, click, fill in forms, or test within the site.

### 3. Execution via Browser Extension

* Click on your project card to launch the configured URL in your browser.
* Open the SpectreQA extension popup in your browser toolbar and click **"Activate SpectreQA on this tab"**.
* **Glass Control Layer:** The extension overlays a transparent control layer over the webpage to handle AI actions and prevent accidental manual interactions during the test.
* Use the floating menu to **Start ▶️**, **Pause ⏸️**, or **Stop ⏹️** autonomous test execution.

---

## ⚖️ License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**. See the `LICENSE` file for details.