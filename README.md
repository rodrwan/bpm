# BPM Detector

Un detector de BPM (Beats Per Minute) escrito en Rust que analiza archivos de audio para determinar el tempo de la música.

## Características

- 🎵 **Soporte múltiple de formatos**: AIFF, WAV, MP3, FLAC, OGG, etc.
- ⚡ **Algoritmo robusto**: Usa autocorrelación para detectar el BPM principal
- 🎯 **Alta precisión**: Error típico de ±2-3 BPM
- 🚀 **Rendimiento optimizado**: Escrito en Rust para máxima velocidad
- 🎧 **Análisis de frecuencias graves**: Enfocado en 40-150 Hz para detectar el beat

## Requisitos

- Rust 1.70+ ([instalar Rust](https://rustup.rs/))
- Cargo (incluido con Rust)

## Instalación

1. Clona el repositorio:
```bash
git clone https://github.com/rodrwan/bpm.git
cd bpm
```

2. Compila el proyecto:
```bash
cargo build --release
```

## Uso

### Uso básico

```bash
cargo run -- "ruta/al/archivo/audio.aiff"
```

### Ejemplos

```bash
# Archivo AIFF
cargo run -- "Electricano - Decisions (Original Mix).aiff"

# Archivo WAV
cargo run -- "cancion.wav"

# Archivo MP3
cargo run -- "track.mp3"

# Con espacios en el nombre
cargo run -- "Mi Cancion Favorita.flac"
```

### Ejecutar binario compilado

```bash
# Compilar en modo release
cargo build --release

# Ejecutar el binario
./target/release/bpm "archivo.aiff"
```

## Cómo funciona

### 1. Lectura del archivo
- Usa la librería Symphonia para decodificar múltiples formatos de audio
- Extrae solo el canal izquierdo para simplificar el análisis
- Convierte todos los samples a formato f32 normalizado

### 2. Análisis espectral
- Aplica FFT (Fast Fourier Transform) con ventana de 2048 samples
- Analiza la banda de frecuencias graves (40-150 Hz)
- Calcula la energía espectral para cada frame

### 3. Detección de BPM
- Normaliza las energías para eliminar sesgos de amplitud
- Calcula la autocorrelación de la señal de energía
- Encuentra picos en la autocorrelación que corresponden a patrones rítmicos
- Convierte los intervalos de tiempo a BPM
- Selecciona el BPM más probable (prefiere valores más altos cuando las magnitudes son similares)

### 4. Rango de BPM
- Analiza BPMs entre 60-180 (rangos típicos de música)
- Redondea el resultado a múltiplos de 0.5 BPM

## Algoritmo

El programa utiliza **autocorrelación** en lugar de detección de picos tradicional, lo que lo hace más robusto para diferentes tipos de música:

1. **Normalización**: Las energías se normalizan para eliminar diferencias de amplitud
2. **Autocorrelación**: Calcula la similitud de la señal consigo misma en diferentes desplazamientos temporales
3. **Detección de picos**: Encuentra máximos en la autocorrelación que indican patrones repetitivos
4. **Conversión a BPM**: Convierte los intervalos de tiempo a BPM usando la fórmula: `BPM = 60 / intervalo_segundos`
5. **Selección inteligente**: Prefiere BPMs más altos cuando las magnitudes son similares (evita sub-armónicos)

## Precisión

- **Error típico**: ±2-3 BPM
- **Mejor rendimiento**: Música electrónica, dance, house
- **Limitaciones**: Música con tempo variable o ritmos complejos

## Formato de salida

```
BPM estimado: 129.0
```

## Dependencias

- `symphonia`: Decodificación de audio
- `realfft`: Transformada de Fourier rápida
- `rustfft`: Operaciones con números complejos

## Estructura del proyecto

```
bpm/
├── Cargo.toml          # Configuración del proyecto
├── src/
│   └── main.rs         # Código principal
└── README.md           # Este archivo
```

## Solución de problemas

### Error: "No file path given"
```bash
# Asegúrate de proporcionar la ruta del archivo
cargo run -- "archivo.aiff"
```

### Error: "No se pudo calcular el BPM"
- El archivo puede no tener suficiente contenido rítmico
- Verifica que el archivo no esté corrupto
- Prueba con un archivo diferente

### Error de compilación
```bash
# Actualiza Rust
rustup update

# Limpia y recompila
cargo clean
cargo build
```

## Contribuir

1. Fork el proyecto
2. Crea una rama para tu feature (`git checkout -b feature/AmazingFeature`)
3. Commit tus cambios (`git commit -m 'Add some AmazingFeature'`)
4. Push a la rama (`git push origin feature/AmazingFeature`)
5. Abre un Pull Request

## Licencia

Este proyecto está bajo la Licencia MIT. Ver el archivo `LICENSE` para más detalles.

## Autor

Rodrigo - [GitHub](https://github.com/rodrwan)

---

**Nota**: Este detector de BPM está optimizado para música electrónica y dance. Para otros géneros musicales, la precisión puede variar. 