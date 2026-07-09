// src-tauri/src/error_detector.rs

use serde_json::Value;

/// Códigos HTTP que consideramos indicadores inequívocos de error.
/// (Evitamos capturar cualquier número de 3 dígitos para no generar falsos positivos,
/// por ejemplo un precio "$400" o un contador "500 items").
const ERROR_CODES: &[&str] = &[
    "400", "401", "403", "404", "405", "408", "409", "422", "429",
    "500", "502", "503", "504",
];

/// Busca en los elementos de tipo "label" alguna señal de error:
/// - la palabra "error" (insensible a mayúsculas)
/// - un código HTTP conocido, siempre que aparezca junto a contexto textual
///   (para reducir falsos positivos no lo activamos si el label es SOLO el número)
///
/// Devuelve Some((id, content)) del primer label sospechoso encontrado.
pub fn detect_label_error(elements: &Value) -> Option<(String, String)> {
    let arr = elements.as_array()?;

    for el in arr {
        let id = el.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if !id.to_lowercase().contains("label") {
            continue;
        }

        let content = el.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if content.trim().is_empty() {
            continue;
        }

        let lower = content.to_lowercase();

        if lower.contains("error") {
            return Some((id.to_string(), content.to_string()));
        }

        for code in ERROR_CODES {
            if contains_whole_word(&lower, code) {
                return Some((id.to_string(), content.to_string()));
            }
        }
    }
    None
}

/// Verifica que `word` aparezca como token independiente (no como parte de otro número).
/// Ej: "500" matchea en "Error 500 al procesar" pero no en "45001".
fn contains_whole_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| token == word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_label_error_with_error_word() {
        let elements = json!([
            {
                "id": "label-1-login",
                "content": "Error: Usuario no encontrado"
            }
        ]);
        let result = detect_label_error(&elements);
        assert!(result.is_some());
        let (id, content) = result.unwrap();
        assert_eq!(id, "label-1-login");
        assert_eq!(content, "Error: Usuario no encontrado");
    }

    #[test]
    fn test_detect_label_error_with_404() {
        let elements = json!([
            {
                "id": "label-1-login",
                "content": "Error 404: Página no encontrada"
            }
        ]);
        let result = detect_label_error(&elements);
        assert!(result.is_some());
        let (id, content) = result.unwrap();
        assert_eq!(id, "label-1-login");
        assert_eq!(content, "Error 404: Página no encontrada");
    }

    #[test]
    fn test_detect_label_error_with_500() {
        let elements = json!([
            {
                "id": "label-1-login",
                "content": "Error interno del servidor 500"
            }
        ]);
        let result = detect_label_error(&elements);
        assert!(result.is_some());
        let (id, content) = result.unwrap();
        assert_eq!(id, "label-1-login");
        assert_eq!(content, "Error interno del servidor 500");
    }

    #[test]
    fn test_ignore_non_label_elements() {
        let elements = json!([
            {
                "id": "input-1-login",
                "content": "Error: Usuario no encontrado"
            }
        ]);
        let result = detect_label_error(&elements);
        assert!(result.is_none());
    }

    #[test]
    fn test_ignore_false_positive_number() {
        let elements = json!([
            {
                "id": "label-1-login",
                "content": "Total: 500 items"
            }
        ]);
        let result = detect_label_error(&elements);
        assert!(result.is_none());
    }

    #[test]
    fn test_ignore_price() {
        let elements = json!([
            {
                "id": "label-1-login",
                "content": "Precio: $400"
            }
        ]);
        let result = detect_label_error(&elements);
        assert!(result.is_none());
    }

    #[test]
    fn test_contains_whole_word() {
        assert!(contains_whole_word("Error 500", "500"));
        assert!(contains_whole_word("Error 404", "404"));
        assert!(contains_whole_word("404 not found", "404"));
        assert!(!contains_whole_word("45001", "500"));
        assert!(!contains_whole_word("Error 5000", "500"));
    }
}