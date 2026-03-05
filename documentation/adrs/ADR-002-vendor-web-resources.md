# ADR-002: Vendor All Web Resources in `static/` Directory

## Status

Accepted

## Context

All web resources (CSS, JavaScript, fonts, images, videos) must be
self-contained and embedded in the binary to ensure:

- No external CDN dependencies that could fail
- Consistent behavior across environments
- Offline-first operation
- Simpler deployment (single binary)

## Decision

All web resources are stored in `static/` and embedded at compile time:

```
static/
├── fonts/        # Web fonts (e.g., PressStart2P.woff2)
├── images/       # Favicon, icons
├── js/           # JavaScript modules
├── video/        # Embedded videos
├── style.css     # Main stylesheet
└── manifest.json # PWA manifest
```

### Embedding Mechanism

The `static-serve` crate provides the `embed_assets!` macro:

```rust
use static_serve::embed_assets;

// In debug builds
embed_assets!("static", compress = true);

// In release builds  
embed_assets!("static", compress = true);
```

This embeds all files from the `static/` directory directly into the binary. The
generated `static_router()` function serves them at runtime.

### How It Works

1. **`embed_assets!` macro**: Scans the `static/` directory at compile time
2. **File compression**: Files are compressed (gzip/brotli) for smaller binary
   size
3. **Router integration**: Generated `static_router::<State>()` merges into the
   app router
4. **Path mapping**: Files served from `/` root (e.g., `/style.css`,
   `/js/app.js`)

### Container Build

The `Containerfile` copies the static directory:

```dockerfile
COPY static ./static
```

This ensures the embedding macro has access to files during the build.

## Consequences

### Good

- Single binary deployment, no file server needed
- No runtime external dependencies
- Faster page loads (files bundled in binary)
- Works offline

### Bad

- Larger binary size (~couple MB for fonts/media)
- Rebuild required to update static assets
- Cannot change assets without rebuild

## Guidelines

1. **Never use CDN links** - All resources must be in `static/`
2. **Fonts**: Put in `static/fonts/`, reference as `/fonts/filename.woff2`
3. **JavaScript**: Put in `static/js/`, use ES6 modules with
   `<script type="module">`
4. **CSS**: Put in `static/style.css`, load via `<link rel="stylesheet">`
5. **Images**: Put in `static/images/`, reference as `/images/filename.png`

### Asset Pipeline

Source images are stored in `assets/` at the project root. Run `cargo x assets`
to generate optimized versions to `static/`:

```
assets/                    # Source files (original quality)
├── favicon.png           # 1024x1024 original
└── monday.png ...       # Day sprite sheets

static/                   # Generated (web-optimized)
├── favicon.png           # 32x32
├── images/
│   ├── icon-192.png      # PWA icon
│   ├── icon-512.png      # PWA icon
│   └── monday.png ...   # 512x512, Nearest filter for pixel art
```

**Why two directories?**

- `assets/` keeps originals clean for re-generation
- `static/` contains web-ready versions (smaller, optimized)

**Image processing:**

- Favicon/icons: Resized with Lanczos3 filter (smooth)
- Pixel art sprites: Resized with Nearest filter (preserves crisp edges)
- Compression happens automatically via `static-serve`

**Adding new images:**

1. Place source in `assets/`
2. Add to `cargo x assets` command in `x/src/main.rs`
3. Run `cargo x assets` to generate
4. Reference in HTML/CSS normally

## Alternative Considered

### External CDN

- Rejected: Adds dependency on external service, potential failure points
- Would require runtime network access

### Load from filesystem at runtime

- Rejected: More complex deployment, requires file synchronization
- Embedding is simpler and more reliable
