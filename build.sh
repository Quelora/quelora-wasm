#!/bin/bash
# Quelora — quelora-wasm
# Copyright (C) 2026 Germán Zelaya — https://quelora.org
# SPDX-License-Identifier: AGPL-3.0-only
#
# This file is part of Quelora. See the LICENSE file for terms.


# ==========================================
# CONFIGURACIÓN
# ==========================================

RELATIVE_DEST_PATH="../quelora-widget-community/js/worker/pkg"
DEST_DIR=$(realpath "$RELATIVE_DEST_PATH")

# Colores
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# ==========================================
# FUNCIONES
# ==========================================

function optimize_wasm() {
    local WASM_FILE="$1"

    if [ ! -f "$WASM_FILE" ]; then
        echo -e "${RED}❌  No se encontró $WASM_FILE${NC}"
        exit 1
    fi

    echo -e "${BLUE}🧬  Optimizando WASM (${YELLOW}wasm-opt -Oz${BLUE})...${NC}"

    wasm-opt -Oz "$WASM_FILE" -o "${WASM_FILE}.min"

    mv "${WASM_FILE}.min" "$WASM_FILE"
}

function compile_crate() {
    local CRATE_NAME=$1

    echo -e "${BLUE}➡️  Iniciando compilación de: ${YELLOW}$CRATE_NAME${NC}"

    if [ ! -d "$CRATE_NAME" ]; then
        echo -e "${RED}❌  La carpeta $CRATE_NAME no existe.${NC}"
        exit 1
    fi

    cd "$CRATE_NAME" || exit 1

    echo -e "${BLUE}⚙️  Ejecutando wasm-pack...${NC}"

    if wasm-pack build --release --target web --out-dir "$DEST_DIR"; then
        echo -e "${GREEN}✅  $CRATE_NAME compilado correctamente.${NC}"
    else
        echo -e "${RED}❌  Error compilando $CRATE_NAME${NC}"
        exit 1
    fi

    # Detectar archivo wasm generado
    WASM_FILE=$(ls "$DEST_DIR"/*"${CRATE_NAME//-/_}"*_bg.wasm 2>/dev/null | head -n 1)

    if [ -z "$WASM_FILE" ]; then
        echo -e "${RED}❌  No se encontró el .wasm generado para $CRATE_NAME${NC}"
        exit 1
    fi

    optimize_wasm "$WASM_FILE"

    echo -e "${GREEN}📦  WASM final optimizado: ${NC}$(basename "$WASM_FILE")"
    echo -e "${BLUE}📂  Ubicación: ${NC}$DEST_DIR"

    cd ..
    echo "----------------------------------------"
}

# ==========================================
# MENU
# ==========================================

clear
echo -e "${YELLOW}🔨  BUILDER DE QUELORA WASM${NC}"
echo "Ruta de destino configurada: $DEST_DIR"
echo "----------------------------------------"
echo "Selecciona qué compilar:"
echo "1. image-processor"
echo "2. markdown-parser"
echo "3. Todo (All)"
echo "----------------------------------------"
read -p "Opción (1-3): " option
echo ""

case $option in
    1)
        compile_crate "image-processor"
        ;;
    2)
        compile_crate "markdown-parser"
        ;;
    3)
        echo -e "${YELLOW}🚀  Compilando TODO...${NC}"
        compile_crate "image-processor"
        compile_crate "markdown-parser"
        echo -e "${GREEN}✨  Proceso global finalizado.${NC}"
        ;;
    *)
        echo -e "${RED}❌  Opción no válida.${NC}"
        exit 1
        ;;
esac
