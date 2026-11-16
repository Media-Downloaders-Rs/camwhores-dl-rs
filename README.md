# CamWhores DL

Descargador independiente de etiquetas públicas de CamWhores. Pega enlaces como `XXX`. Recorre todas las páginas del catálogo, descarga solo el MP4 firmado declarado por el reproductor y deja los vídeos privados o no disponibles como pendientes.

## Compilar

Instala Rust estable y las herramientas MSVC para Windows.

```powershell
cargo build --release --locked
powershell -NoProfile -File tools/install-build.ps1
```

El ejecutable es `camwhores-dl.exe`. Los archivos y registros locales permanecen en `downloads/`, fuera de Git.

## Verificar

```powershell
cargo test --locked --offline
cargo fmt --check
cargo clippy --locked --offline --all-targets --all-features -- -D warnings
powershell -File tools/verify-standalone.ps1
```

## Licencia y uso responsable

El código propio se distribuye bajo la [licencia MIT](LICENSE). El propósito educativo y las responsabilidades de uso se describen en [USAGE_NOTICE.md](USAGE_NOTICE.md). Esta licencia no concede derechos sobre contenido ajeno.
