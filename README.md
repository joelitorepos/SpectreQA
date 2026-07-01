# 🔍 SpectreQA

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Built with Bun](https://img.shields.io/badge/Built%20with-Bun-fbf0df?style=flat&logo=bun&logoColor=black)](https://bun.sh/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-v19-61dafb?style=flat&logo=react&logoColor=black)](https://react.dev/)

**SpectreQA** es una aplicación de escritorio diseñada exclusivamente para realizar **testing de aplicaciones web de extremo a extremo (E2E) impulsado por Inteligencia Artificial**. Mediante una arquitectura híbrida que conecta una app de escritorio ligera (Tauri + React), un modelo de lenguaje (Ollama) y una extensión de navegador, SpectreQA toma el control de tu entorno web para automatizar y validar flujos interactivos complejos basándose en tus prompts.

---

## 🛠️ Requisitos Previos

Para poder utilizar e interactuar con SpectreQA, necesitarás contar con tres componentes esenciales instalados en tu sistema:

1. **La Aplicación de Escritorio:** (Este repositorio).
2. **Ollama:** Es mandatorio tenerlo instalado y corriendo en tu sistema, **incluso si no planeas utilizar modelos locales**.
3. **La Extensión para el Navegador:** Dado que aún no se encuentra publicada en las tiendas oficiales, debes clonar e instalar manualmente su repositorio desde:  
   🔗 [https://github.com/joelitorepos/extension-spectreqa](https://github.com/joelitorepos/extension-spectreqa) *(Cárgala en tu navegador basado en Chromium activando el "Modo de desarrollador" y seleccionando la carpeta del proyecto clonado)*.

---

## 🚀 Comandos de Desarrollo y Compilación

Este proyecto utiliza **Bun** como entorno de ejecución y gestor de paquetes. Desde la raíz del proyecto, puedes ejecutar los siguientes scripts:

### Probar la interfaz frontend en el navegador
```bash
bun run dev

```

### Compilar y ejecutar la App de Escritorio (Modo Desarrollo)

Levanta la ventana nativa de Tauri vinculada al entorno interactivo:

```bash
bun run tauri dev

```

### Crear el instalador/descargable de producción

Genera los binarios empaquetados optimizados para distribución:

```bash
bun run tauri build

```

* **En Linux (Debian/Ubuntu/Mint):** El paquete final se generará en la ruta:
```bash
cd src-tauri/target/release/bundle/deb/

```


Encontrarás los archivos `SpectreQA_0.1.0_amd64` y `SpectreQA_0.1.0_amd64.deb`. Puedes instalarlo en tu sistema corriendo:
```bash
sudo apt install ./SpectreQA_0.1.0_amd64.deb

```


* **En Windows:** Al compilar bajo este entorno mediante su sistema de *dual boot*, se generará un instalador ejecutable (`.exe`) dentro de la estructura homóloga de compilación de Windows:
```text
src-tauri\target\release\bundle\nsis\

```



---

## 💻 Guía de Uso paso a paso

### 1. Configuración de un Proyecto

Al abrir la aplicación de escritorio, podrás dar de alta los entornos de las aplicaciones web que deseas auditar:

* **Si el proyecto está en producción:** Ingresa únicamente el Nombre identificador y el Dominio/URL pública de tu aplicación.
* **Si el proyecto está en entorno local:** Ingresa el Nombre, el Host y Puerto (ej. `http://localhost:3000`), la ruta absoluta de la **carpeta raíz de tu proyecto** en tu disco, y el **comando específico para levantar el proyecto** (ej. `npm run dev` o `bun run dev`).

### 2. Definir el Objetivo de la IA (Prompt)

En la interfaz principal de SpectreQA verás las tarjetas (*cards*) de tus proyectos configurados.

* Pasa el cursor por encima de la tarjeta del proyecto para revelar un **ícono en forma de archivo**.
* Haz clic en él para abrir el panel de configuración donde ingresarás el **prompt** detallado. Aquí le dictas las instrucciones exactas a la IA sobre qué debe buscar, probar o validar dentro de tu aplicación.

### 3. Activación del "Vidrio" (Glass) y Ejecución del Test

* Haz clic directamente sobre la tarjeta de tu proyecto. Esto abrirá automáticamente tu navegador web predeterminado en la ruta configurada.
* Abre el popup de la extensión de SpectreQA en tu navegador y haz clic en el botón **"Activar SpectreQA en esta pestaña"**.

> ⚠️ **¿Qué pasa al activar la extensión?**
> Al activarse, un contenedor `div` invisible recubrirá por completo la página web, actuando como un "vidrio". **Todas las acciones normales del usuario se deshabilitarán**, impidiendo que interactúes manualmente con el sitio para evitar interferir con las pruebas.

* En pantalla aparecerá un **menú flotante desplegable**. Desde este menú flotante podrás:
* Activar o desactivar temporalmente el "vidrio" si requieres control manual.
* Usar los botones de **Correr ▶️**, **Pausar ⏸️** y **Detener ⏹️** para controlar el flujo de la IA, permitiéndole empezar a interactuar de forma autónoma con tu programa siguiendo el prompt que definiste previamente.



---

## ⚖️ Licencia

Este proyecto está licenciado bajo la licencia **GNU Affero General Public License v3.0 (AGPLv3)**. Consulta el archivo `LICENSE` para obtener más detalles. Cualquier modificación o distribución comercial/en red de este software requiere la liberación inmediata de su respectivo código fuente.

```

### ¿Qué aporta esta versión?
1. **Claridad técnica:** Explica detalladamente cómo funciona la extensión y por qué se bloquea la pantalla con el `div`, lo cual reducirá reportes de errores de usuarios que piensen que la app se congeló.
2. **Orden de compilación:** Aclara exactamente las rutas de salida de los binarios para evitar confusiones de ruta entre Linux Mint y Windows 11.
3. **Atractivo visual:** Los *badges* de Bun, Tauri, React y AGPLv3 al inicio le dan un aspecto de proyecto maduro y profesional en tu perfil de GitHub.

```