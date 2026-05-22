/*
 * Quelora — quelora-wasm
 * Copyright (C) 2026 Germán Zelaya — https://quelora.org
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * This file is part of Quelora. See the LICENSE file for terms.
 */

/* filepath: packages/quelora-wasm/image-processor/src/lib.rs */
use wasm_bindgen::prelude::*;
use image::imageops::FilterType;
use std::io::Cursor;
use image::GenericImageView;

#[wasm_bindgen]
pub fn process_image(
    file_bytes: &[u8],
    x: u32,
    y: u32,
    width: Option<u32>,  // Ahora opcional
    height: Option<u32>, // Ahora opcional
    _output_format: &str,
) -> Result<Vec<u8>, JsValue> {

    // 1. Cargar
    let img = image::load_from_memory(file_bytes)
        .map_err(|_| JsValue::from_str("Error loading image"))?;

    let (orig_w, orig_h) = img.dimensions();

    // 2. Determinar dimensiones de recorte (si no hay width/height, usamos el resto de la imagen)
    let cw = width.unwrap_or(orig_w.saturating_sub(x));
    let ch = height.unwrap_or(orig_h.saturating_sub(y));

    // 3. Procesar
    let cropped_img = if x == 0 && y == 0 && cw == orig_w && ch == orig_h {
        img // Es la imagen completa
    } else {
        let safe_w = if x + cw > orig_w { orig_w - x } else { cw };
        let safe_h = if y + ch > orig_h { orig_h - y } else { ch };
        img.crop_imm(x, y, safe_w, safe_h)
    };

    // 4. Redimensionar solo si se pasaron AMBOS parámetros
    let processed_img = match (width, height) {
        (Some(w), Some(h)) => cropped_img.resize_exact(w, h, FilterType::Triangle),
        _ => cropped_img, // Devuelve el recorte (o original) sin estirar
    };

    // 5. Codificar (JPEG 75%)
    let mut bytes = Vec::new();
    processed_img
        .write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Jpeg(75))
        .map_err(|_| JsValue::from_str("Error encoding JPEG"))?;

    Ok(bytes)
}

#[wasm_bindgen]
pub fn resize_to_max_width(
    file_bytes: &[u8],
    max_width: u32,
) -> Result<Vec<u8>, JsValue> {
    let img = image::load_from_memory(file_bytes)
        .map_err(|_| JsValue::from_str("Error loading image"))?;

    let (orig_w, _) = img.dimensions();

    if orig_w <= max_width {
        return Ok(file_bytes.to_vec());
    }

    let resized_img = img.resize(max_width, u32::MAX, FilterType::Triangle);

    let mut bytes = Vec::new();
    resized_img
        .write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Jpeg(75))
        .map_err(|_| JsValue::from_str("Error encoding JPEG"))?;

    Ok(bytes)
}