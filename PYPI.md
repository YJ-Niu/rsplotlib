# rsplotlib

A high-performance Python plotting library powered by Rust, with a Matplotlib-compatible API.

## Key Features

- **Matplotlib-Compatible API**: Low migration cost for existing Matplotlib users
- **Rust-Powered Performance**: Offload rendering and batch operations to Rust for significant speed improvements
- **Rich Chart Types**: Line plots, scatter plots, bar charts, histograms, box plots, pie charts, heatmaps, and more
- **Per-Point Scatter Colors & Sizes**: Rust-level batch processing, zero Python loop overhead
- **Math Text Rendering**: LaTeX-style `$...$` expressions for titles, labels, and annotations
- **Colormaps & Colorbar**: 50+ built-in colormaps with `LogNorm` support
- **Image I/O**: `imshow`, `imread`, `imsave` with RGB/RGBA support
- **Cross-Platform Font Resolution**: Automatic system font detection with CJK support
- **High-Quality Output**: PNG and SVG with customizable DPI

## Installation

```bash
pip install rsplotlib
```

Prebuilt wheels available for Linux (x86_64/aarch64), macOS (universal2), and Windows (x64) across Python 3.10-3.14.

## Quick Start

```python
from rsplotlib import pyplot as plt
from rsplotlib.pylab import mpl

# Optional: Configure fonts
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Arial']

fig, ax = plt.subplots(figsize=(8, 6))
ax.plot([1, 2, 3], [1, 4, 9], label='Quadratic')
ax.set_title('Basic Line Plot')
ax.legend()
fig.savefig('plot.png', dpi=300)
```

## Version v0.3.5

### Improvements
- Increased default text font size by 10% for better readability
- Fixed text alignment default values and Python interface parameter mismatch
- Code cleanup and dependency optimization

### Previous Versions
- v0.3.4: Legend ellipsis and layout improvements
- v0.3.3: Subplot layout fixes, dependency refactoring (removed rsnumpy)
- v0.3.2: Scipy interpolation simulation
- v0.3.1: Subplot spacing adjustments
- v0.3.0: Major API and architecture refactoring

## Documentation

- [GitHub Repository](https://github.com/YJ-Niu/rsplotlib)
- [README](https://github.com/YJ-Niu/rsplotlib/blob/dev/README.md)
- [Release Notes](https://github.com/YJ-Niu/rsplotlib/blob/dev/RELEASE_NOTES.md)

## License

MIT License