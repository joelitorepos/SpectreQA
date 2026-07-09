#!/usr/bin/env python3
import os
import re
from pathlib import Path

def unbundle_application_code():
    # Buscamos el archivo generado (puedes cambiarlo a full_code.txt si tuviera ese nombre)
    input_file = Path("codigo_completo.txt")
    
    if not input_file.exists():
        print(f"❌ Error: No se encontró el archivo '{input_file}' en el directorio actual.")
        return

    print(f"🔍 Leyendo respaldo desde {input_file.as_posix()}...")

    # Expresiones regulares para detectar las etiquetas de inicio y fin
    inicio_regex = re.compile(r"^--- INICIO ARCHIVO: (.+) ---$")
    fin_regex = re.compile(r"^--- FIN ARCHIVO: (.+) ---$")

    files_restored = 0
    current_file_path = None
    current_content = []
    inside_file = False

    try:
        with open(input_file, "r", encoding="utf-8") as infile:
            for line in infile:
                # Comprobar si es línea de inicio
                inicio_match = inicio_regex.match(line.strip())
                if inicio_match:
                    current_file_path = Path(inicio_match.group(1))
                    inside_file = True
                    current_content = []
                    continue

                # Comprobar si es línea de fin
                fin_match = fin_regex.match(line.strip())
                if fin_match:
                    if inside_file and current_file_path:
                        # Unir el contenido acumulado
                        file_text = "".join(current_content)
                        
                        # Si el archivo original terminaba con salto de línea extra por el script, se lo quitamos
                        if file_text.endswith("\n"):
                            file_text = file_text[:-1]

                        # Asegurar que las carpetas contenedoras existan
                        current_file_path.parent.mkdir(parents=True, exist_ok=True)
                        
                        # Escribir o truncar el archivo existente
                        with open(current_file_path, "w", encoding="utf-8") as outfile:
                            outfile.write(file_text)
                        
                        print(f"🛠️ Restaurado/Truncado: {current_file_path.as_posix()}")
                        files_restored += 1
                    
                    inside_file = False
                    current_file_path = None
                    continue

                # Si estamos dentro de un archivo, acumulamos las líneas
                if inside_file:
                    current_content.append(line)

        print(f"\n🚀 ¡Restauración completada con éxito!")
        print(f"✨ Se actualizaron {files_restored} archivos basados en el respaldo.")

    except Exception as e:
        print(f"❌ Error crítico al procesar el archivo de respaldo: {e}")

if __name__ == "__main__":
    unbundle_application_code()