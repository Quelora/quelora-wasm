/*
 * Quelora — quelora-wasm
 * Copyright (C) 2026 Germán Zelaya — https://quelora.org
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * This file is part of Quelora. See the LICENSE file for terms.
 */

use wasm_bindgen::prelude::*;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use regex_lite::Regex;
use once_cell::sync::Lazy;

// ─────────────────────────────────────────────────────────────
// Static Regex
// ─────────────────────────────────────────────────────────────

static GIPHY_REGEX: Lazy<Regex> = Lazy::new(|| {
    // Captura IDs alfanuméricos solamente. Es seguro por definición.
    // Grupo 1: notación markdown  ![...](giphy|ID)
    // Grupo 2: URL directa        https://media.giphy.com/media/ID/giphy.gif
    // Grupo 3: URL de página      https://giphy.com/gifs/...-ID
    Regex::new(
        r"!\[[^\]]*\]\(giphy\|([a-zA-Z0-9]+)(?:\|[^)]*)?\)|https?://media\.giphy\.com/media/([a-zA-Z0-9]+)/giphy\.gif|https?://giphy\.com/(?:gifs|stickers)/(?:[a-zA-Z0-9\-]+-)?([a-zA-Z0-9]+)"
    ).unwrap()
});

static MENTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    // Captura alfanuméricos y guión bajo. Seguro por definición.
    Regex::new(r"@([a-zA-Z0-9_]+)").unwrap()
});

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    // [^\s] captura todo lo que no sea espacio, INCLUYENDO comillas.
    // Por eso es vital escapar las comillas en el reemplazo más abajo.
    Regex::new(r"(https?://[^\s]+)").unwrap()
});

// ─────────────────────────────────────────────────────────────
// Helpers de Seguridad
// ─────────────────────────────────────────────────────────────

/// Valida protocolo (Anti-Javascript:)
fn is_safe_href(href: &str) -> bool {
    let lower = href.trim().to_lowercase();
    lower.starts_with("http://") ||
    lower.starts_with("https://") ||
    lower.starts_with("mailto:") ||
    lower.starts_with("#")
}

/// Escapa comillas (Anti-Attribute Injection).
/// Convierte `"` en `&quot;` para que no rompan el atributo `href="..."`.
fn escape_attr(text: &str) -> String {
    text.replace("\"", "&quot;")
}

/// Construye el HTML canónico para un GIF de Giphy dado su ID alfanumérico.
/// Centraliza el formato para que `Tag::Image` y `Event::Text` produzcan
/// salida idéntica.
fn giphy_html(id: &str) -> String {
    format!(
        r#"<div class="gif-container"><img src="https://media.giphy.com/media/{}/giphy.gif" alt="GIF" loading="lazy" style="max-width: 50%; height: auto; display: block; border-radius: 8px;" /></div>"#,
        id
    )
}

// ─────────────────────────────────────────────────────────────
// Markdown → HTML
// ─────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn parse_markdown(input_text: &str) -> String {
    // Phase 0: Decode (Seguro porque filtramos HTML crudo después)
    let text_decoded = input_text
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#039;", "'");

    // Phase 1: Clean Lines
    let lines: Vec<&str> = text_decoded.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut clean_text = String::with_capacity(text_decoded.len());
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('>') {
            let content = line.trim_start_matches('>').trim();
            clean_text.push_str("> ");
            clean_text.push_str(content);
        } else {
            clean_text.push_str(line);
        }
        if i < lines.len() - 1 {
            clean_text.push('\n');
        }
    }

    // Phase 2: Parse
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&clean_text, options);

    // ── Estado mutable ──────────────────────────────────────────────────────
    let mut inside_link      = false;
    let mut inside_giphy_img = false; // true mientras estamos dentro de Tag::Image con giphy|
    let mut list_depth: i32  = 0;
    const MAX_LIST_DEPTH: i32 = 5;

    let events = parser.map(|event| match event {

        // ─── Imágenes Giphy: intercepción temprana ────────────────────────────
        //
        // pulldown-cmark parsea `![GIF](giphy|ID)` como:
        //   Event::Start(Tag::Image { dest_url: "giphy|ID", .. })
        //   Event::Text("GIF")          ← texto alt
        //   Event::End(TagEnd::Image)
        //
        // Si lo dejamos pasar, el renderer emite `<img src="giphy%7CID">` —
        // la `|` URL-encoded — y GIPHY_REGEX nunca llega a ejecutarse porque
        // el evento nunca llega a Event::Text en este flujo.
        //
        // Solución: detectar `dest_url` que empieza con "giphy|" en
        // Tag::Image, emitir el HTML correcto de inmediato, y activar la
        // bandera `inside_giphy_img` para suprimir los dos eventos siguientes
        // (alt text y cierre) que el parser todavía va a emitir.

        Event::Start(Tag::Image { dest_url, .. }) => {
            let url_str = dest_url.as_ref();

            if url_str.starts_with("giphy|") {
                // Extraer el ID — todo lo que sigue al primer '|', ignorando
                // cualquier parámetro adicional separado por otro '|'.
                let raw_id = &url_str["giphy|".len()..];
                let id = raw_id.split('|').next().unwrap_or("").trim();

                // Validar que el ID sea estrictamente alfanumérico para evitar
                // cualquier inyección aunque el origen sea el propio cliente.
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()) {
                    inside_giphy_img = true;
                    return Event::Html(CowStr::from(giphy_html(id)));
                }
            }

            // Imagen normal (no Giphy): suprimir para no exponer rutas de
            // archivo arbitrarias en src. Las imágenes externas se manejan
            // mediante las URLs capturadas por URL_REGEX en Event::Text.
            Event::Html(CowStr::from(""))
        }

        Event::End(TagEnd::Image) => {
            if inside_giphy_img {
                inside_giphy_img = false;
                // El HTML del gif ya fue emitido en Start; no emitir nada aquí.
                return Event::Html(CowStr::from(""));
            }
            Event::Html(CowStr::from(""))
        }

        // ─── Limpieza y Seguridad HTML ───────────────────────────────────────
        Event::Start(Tag::Heading { .. }) => Event::Html(CowStr::from("<p><strong>")),
        Event::End(TagEnd::Heading(_))   => Event::Html(CowStr::from("</strong></p>")),
        Event::Html(_) | Event::InlineHtml(_) => Event::Html(CowStr::from("")), // Borrar HTML crudo

        // ─── Estructura y Anti-DoS ───────────────────────────────────────────
        Event::Start(Tag::List(_)) => {
            list_depth += 1;
            Event::Html(CowStr::from(""))
        }
        Event::End(TagEnd::List(_)) => {
            list_depth -= 1;
            Event::Html(CowStr::from(""))
        }
        Event::Start(Tag::Item) => {
            if list_depth > MAX_LIST_DEPTH { Event::Html(CowStr::from("")) }
            else { Event::Html(CowStr::from("<span class='ql-list-item'>")) }
        }
        Event::End(TagEnd::Item) => {
            if list_depth > MAX_LIST_DEPTH { Event::Html(CowStr::from("")) }
            else { Event::Html(CowStr::from("</span><br>")) }
        }
        Event::Start(Tag::BlockQuote(_)) => Event::Html(CowStr::from(r#"<blockquote class="ql-quote">"#)),

        // ─── SEGURIDAD CRÍTICA: Enlaces Markdown ─────────────────────────────
        Event::Start(Tag::Link { dest_url, title, .. }) => {
            inside_link = true;

            // 1. Validar Protocolo
            let url_checked = if is_safe_href(&dest_url) {
                dest_url
            } else {
                CowStr::from("#unsafe-link-blocked")
            };

            // 2. Escapar Comillas
            let url_safe   = escape_attr(&url_checked);
            let title_safe = escape_attr(&title);

            let title_attr = if title.is_empty() {
                String::new()
            } else {
                format!(" title=\"{}\"", title_safe)
            };

            let html = format!(
                r#"<a href="{}" class="ql-link-external" target="_blank" rel="noopener noreferrer"{}>🔗 Open Link"#,
                url_safe, title_attr
            );

            Event::Html(CowStr::from(html))
        }

        Event::End(TagEnd::Link) => {
            inside_link = false;
            Event::Html(CowStr::from("</a>"))
        }

        // ─── Procesamiento de Texto ──────────────────────────────────────────
        Event::Text(text) => {
            // Suprimir el texto alt que pulldown-cmark emite dentro de una
            // imagen Giphy ya procesada — el gif ya fue renderizado en Start.
            if inside_giphy_img {
                return Event::Html(CowStr::from(""));
            }

            if inside_link { return Event::Text(text); }

            // Giphy via URL directa en texto plano (fallback para comentarios
            // migrados que usen la URL completa en lugar de la notación markdown).
            if let Some(caps) = GIPHY_REGEX.captures(&text) {
                if let Some(id_match) = caps.get(1).or(caps.get(2)).or(caps.get(3)) {
                    let id = id_match.as_str(); // Alphanum only
                    return Event::Html(CowStr::from(giphy_html(id)));
                }
            }

            // Mentions (Regex seguro, escape preventivo)
            if MENTION_REGEX.is_match(&text) {
                let replaced = MENTION_REGEX.replace_all(&text, |caps: &regex_lite::Captures| {
                    let username = caps.get(1).unwrap().as_str();
                    let safe_user = escape_attr(username);
                    format!(
                        r##"<a href="#mention:{}" data-callback="mention:{}">@{}</a>"##,
                        safe_user, safe_user, safe_user
                    )
                });

                // Procesar URLs dentro del resto del texto (Recursividad simulada)
                if URL_REGEX.is_match(&replaced) {
                    let replaced_url = URL_REGEX.replace_all(&replaced, |caps: &regex_lite::Captures| {
                        let url = caps.get(1).unwrap().as_str();
                        if is_safe_href(url) {
                            let safe_url = escape_attr(url);
                            format!(
                                r#"<a href="{}" class="ql-link-external" target="_blank" rel="noopener noreferrer">🔗 Open Link</a>"#,
                                safe_url
                            )
                        } else {
                            url.to_string().replace("<", "&lt;").replace(">", "&gt;")
                        }
                    });
                    return Event::Html(CowStr::from(replaced_url.to_string()));
                }
                return Event::Html(CowStr::from(replaced.to_string()));
            }

            // Autolinks (URLs sueltas)
            if URL_REGEX.is_match(&text) {
                let replaced = URL_REGEX.replace_all(&text, |caps: &regex_lite::Captures| {
                    let url = caps.get(1).unwrap().as_str();
                    if is_safe_href(url) {
                        let safe_url = escape_attr(url);
                        format!(
                            r#"<a href="{}" class="ql-link-external" target="_blank" rel="noopener noreferrer">🔗 Open Link</a>"#,
                            safe_url
                        )
                    } else {
                        url.to_string().replace("<", "&lt;").replace(">", "&gt;")
                    }
                });
                return Event::Html(CowStr::from(replaced.to_string()));
            }

            Event::Text(text)
        }

        _ => event,
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, events);

    html_output.trim().to_string()
}