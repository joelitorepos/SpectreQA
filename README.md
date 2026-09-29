# 🔍 SpectreQA

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Built with Bun](https://img.shields.io/badge/Built%20with-Bun-fbf0df?style=flat&logo=bun&logoColor=black)](https://bun.sh/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-v19-61dafb?style=flat&logo=react&logoColor=black)](https://react.dev/)
[![Chrome Web Store](https://img.shields.io/badge/Chrome_Web_Store-SpectreQA-4285F4?style=flat&logo=googlechrome&logoColor=white)](https://chromewebstore.google.com/detail/spectreqa/eonjhhhdhlmlhbcmccenadolnkpingpb)

**SpectreQA** es un ecosistema de automatización de pruebas de extremo a extremo (E2E) e interacción web impulsado por Inteligencia Artificial. Mediante una arquitectura híbrida compuesta por una aplicación de escritorio ligera (Tauri + React), una extensión de navegador y soporte para LLMs (nube o locales), SpectreQA audita y valida flujos interactivos complejos en aplicaciones web basándose en instrucciones de lenguaje natural (*prompts*).

---

## 🛠️ Requisitos e Instalación

Para utilizar SpectreQA necesitas:

1. **La Aplicación de Escritorio:** Descarga el binario compilado o compílalo desde este repositorio.
2. **La Extensión para el Navegador:** Disponible directamente en la **Chrome Web Store**:  
   🔗 [Instalar SpectreQA Extension](https://chromewebstore.google.com/detail/spectreqa/eonjhhhdhlmlhbcmccenadolnkpingpb)
3. *(Opcional)* **Ollama:** Si prefieres procesar las pruebas de forma 100% privada ejecutando modelos de lenguaje locales en tu máquina, instala [Ollama](https://ollama.com/) y asegúrate de tenerlo iniciado.

---

## 🚀 Comandos de Desarrollo y Compilación

Este proyecto utiliza **Bun** como entorno de ejecución y gestor de paquetes.

### Modo desarrollo (Frontend en navegador)
```bash
bun run dev
```

### Ejecutar App de Escritorio (Modo Desarrollo)
Levanta la ventana nativa de Tauri con *Hot Reload*:
```bash
bun run tauri dev
```

### Crear instaladores de producción
Genera los binarios empaquetados optimizados para distribución:
```bash
bun run tauri build
```

- **Linux (Debian/Ubuntu/Mint):** Genera paquetes `.deb` e imagen binaria en:
  ```bash
  cd src-tauri/target/release/bundle/deb/
  sudo apt install ./SpectreQA_0.1.0_amd64.deb
  ```

- **Windows:** Genera un instalador ejecutable (`.exe` / `.msi`) en:
  ```text
  src-tauri\target\release\bundle\nsis\
  ```

---

## 💻 Guía de Uso paso a paso

### 1. Registrar un Proyecto
Al abrir SpectreQA, registra el entorno de la aplicación web que deseas probar:
* **Entorno Público / Producción:** Ingresa el Nombre y la URL pública (ej. `https://mi-app.com`).
* **Entorno Local:** Ingresa el Nombre, Host/Puerto (ej. `http://localhost:3000`), la ruta raíz en tu disco y el comando para iniciarlo (ej. `npm run dev`).

### 2. Definir el Objetivo de la Prueba (*Prompt*)
* Pasa el cursor sobre la tarjeta del proyecto en SpectreQA y haz clic en el ícono de configuración/archivo.
* Redacta el **prompt** con las instrucciones detalladas para la IA sobre qué elementos validar, hacer clic, llenar formularios o probar dentro del sitio.

### 3. Ejecución con la Extensión
* Haz clic sobre la tarjeta de tu proyecto para abrir la URL configurada en el navegador.
* Abre el popup de la extensión de SpectreQA en la barra del navegador y selecciona **"Activar SpectreQA en esta pestaña"**.
* **Capa de Control "Glass":** La extensión recubrirá la pestaña con una capa transparente (*overlay*) para gestionar los eventos de la IA y evitar interferencias manuales accidentales durante el testeo.
* Utiliza el menú flotante para **Iniciar ▶️**, **Pausar ⏸️** o **Detener ⏹️** la ejecución de la prueba autónoma.

---

## ⚖️ Licencia

Este proyecto está licenciado bajo la licencia **GNU Affero General Public License v3.0 (AGPLv3)**. Consulta el archivo `LICENSE` para más detalles.