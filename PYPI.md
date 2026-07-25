# rsplotlib

A high-performance Python plotting library powered by Rust, with a Matplotlib-compatible API.

## Key Features

- **Matplotlib-Compatible API**: Low migration cost for existing Matplotlib users
- **scikit-rf Integration**: Full support for Network objects with frequency band slicing (e.g., `ring_slot['82-90ghz']`)
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

### scikit-rf Integration

```python
import skrf as rf
from skrf.data import ring_slot
import rsplotlib.pyplot as plt

rf.stylely()
ring_slot.s11.plot_s_db(label='Full Band Response')
# Frequency band slicing: select 82-90 GHz range
ring_slot.s11['82-90ghz'].plot_s_db(lw=3, label='Band of Interest')
plt.legend()
plt.savefig('s_parameters.png')
```

Frequency band slicing supports human-readable strings like `'80-90ghz'`, `'1-2ghz'`, or `'500mhz'`. The sliced Network object retains all plotting capabilities (`plot_s_db`, `plot_s_mag`, `plot_s_smith`, etc.) with full matplotlib-compatible keyword arguments (`lw`, `ls`, `color`, `marker`, etc.).

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
