# rsplotlib

> A high-performance Python plotting library powered by Rust, with a Matplotlib-compatible API

[![Python](https://img.shields.io/badge/Python-3.10%2B-blue)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2024-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![PyO3](https://img.shields.io/badge/PyO3-0.29-2c2d72)](https://pyo3.rs/)
[![plotters](https://img.shields.io/badge/plotters-0.3-7d5cff)](https://github.com/plotters-rs/plotters)

---

## Table of Contents

- [Introduction](#introduction)
- [Core Features](#core-features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Feature Reference](#feature-reference)
- [API Reference](#api-reference)
- [Performance Advantages](#performance-advantages)
- [Project Structure](#project-structure)
- [Development & Contributing](#development--contributing)
- [Font Configuration](#font-configuration)
- [License](#license)
- [Acknowledgments](#acknowledgments)

---

## Introduction

**rsplotlib** is a cross-language Python plotting library whose core rendering engine is written entirely in Rust, providing a Matplotlib-compatible Python API through PyO3. The project aims to maximize compatibility with existing Matplotlib code while leveraging Rust's memory safety and zero-cost abstractions to deliver significant performance improvements.

### Design Philosophy

- **API First**: Maintain a highly compatible interface with Matplotlib for low migration cost
- **Performance Matters**: Offload performance-critical paths (rendering, batch operations) to Rust
- **Zero Extra Dependencies**: No native Matplotlib installation required - just a Python interpreter
- **Cross-Platform Consistency**: Identical rendering quality on macOS, Linux, and Windows

---

## Core Features

### Rich Chart Types

- **Basic Charts**: Line plots, scatter plots, bar charts, horizontal bar charts
- **Statistical Charts**: Histograms, box plots, pie charts, error bars, stem plots, step plots
- **Advanced Charts**: Stacked area plots, heatmap/image display, fill-between regions

### Annotation & Reference Elements

- **Reference Lines**: Horizontal (`axhline`), vertical (`axvline`)
- **Span Highlighting**: Horizontal span (`axhspan`), vertical span (`axvspan`)
- **Arbitrary-Slope Reference Lines**: `axline` - draw a line through any two points, extending across the plot
- **Batch Lines**: `hlines` / `vlines` - Rust-level batch implementation, no Python `for` loops
- **Text Annotations**: `text`, `title`, arrowed `annotate` (with `arrowprops` support)

### Coordinates & Layout

- Multi-subplot support: `subplots()`, `subplot()`, grid layouts
- Twin axes: `twinx()` / `twiny()`
- Log-scale axes: `semilogx()`, `semilogy()`, `loglog()`
- Grid display, legends, axis limits, tick customization

### Output & Resolution

- **PNG**: Bitmap output with DPI control
- **SVG**: Scalable vector output for lossless scaling
- **Custom DPI**: `savefig(filename, dpi=300)` supports arbitrary resolution
- **DPI in PNG Metadata**: DPI written directly into PNG metadata for publication use

### Scatter Plot Enhancements

- **Per-Point Colors**: `ax.scatter(x, y, c=['red', 'blue', 'green'])`
- **Per-Point Sizes**: `ax.scatter(x, y, s=[10, 20, 30, 40])`
- **Independent Color + Size**: Implemented at Rust level - zero Python loop overhead

### Style Management

- **`plt.style` Module**: Matplotlib-compatible style interface
- **`plt.style.use()`**: Apply preset styles
- **`plt.style.available`**: Query available styles
- **`plt.rcParams`**: Global parameter configuration (font family, size, etc.)

### Fonts & Internationalization

- **Cross-Platform Font Resolution**: Automatically finds available system fonts
- **CJK Support**: Built-in detection for Chinese/Japanese/Korean fonts
- **Custom Font Registration**: `register_sans_serif_font(path)` - register any font file

---

## Installation

### Prerequisites

| Dependency | Version | Notes                                   |
| ---------- | ------- | --------------------------------------- |
| Python     | 3.10+   | CPython implementation                  |
| Rust       | 1.70+   | Required only when building from source |
| maturin    | 1.13+   | Rust-Python package build tool          |

### Method 1: Install from PyPI (recommended)

Prebuilt wheels are published for Linux (x86_64/aarch64), macOS (universal2), and Windows (x64) across Python 3.10-3.14. No Rust toolchain required:

```bash
pip install rsplotlib
```

### Method 2: Build from source with maturin (recommended for developers)

```bash
# 1. Clone the repository
git clone https://github.com/YJ-Niu/rsplotlib.git
cd rsplotlib

# 2. Build and install (compiles Rust, installs to current Python env)
pip install maturin
maturin develop --release
```

### Method 3: Build a wheel package

```bash
# Use the project build script (works on macOS/Linux; run under Git Bash/WSL on Windows)
./build_wheel.sh

# Install the generated wheel
pip install target/wheels/rsplotlib-*.whl
```

### Method 4: Rust extension only (for debugging)

```bash
# Compile the Rust cdylib
cargo build --release

# Copy the compiled artifact to the correct Python extension module name
# macOS:
cp target/release/librsplotlib.dylib python/rsplotlib/rsplotlib.cpython-39-darwin.so

# Linux:
# cp target/release/librsplotlib.so python/rsplotlib/rsplotlib.cpython-39-x86_64-linux-gnu.so

# Use from the source directory
export PYTHONPATH="$PWD/python:$PYTHONPATH"
```

### Verify Installation

```python
>>> import rsplotlib.pyplot as plt
>>> fig, ax = plt.subplots()
>>> ax.plot([1, 2, 3], [1, 4, 9])
>>> fig.savefig('test.png')
# If test.png is generated, installation succeeded
```

---

## Quick Start

### Basic Usage: Line Plot

```python
from rsplotlib import pyplot as plt
from rsplotlib.pylab import mpl

# Optional: Configure CJK fonts
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Microsoft YaHei', 'Arial']

# Create Figure and Axes
fig, ax = plt.subplots(figsize=(8, 6))

# Plot data
x = [0, 1, 2, 3, 4, 5]
y1 = [1, 2, 4, 8, 16, 32]
y2 = [32, 16, 8, 4, 2, 1]

ax.plot(x, y1, label='Exponential Growth', lw=2.0)
ax.plot(x, y2, label='Exponential Decay', lw=2.0, linestyle='--')

# Decorate the chart
ax.set_title('Basic Line Plot Example')
ax.set_xlabel('Time')
ax.set_ylabel('Value')
ax.legend()
ax.grid(True)

# Save as high-resolution PNG
fig.savefig('line_plot.png', dpi=300)
```

### Scatter Plot: Per-Point Colors and Sizes

```python
import random
from rsplotlib import pyplot as plt

# Generate random data
random.seed(42)
n = 50
x = [random.gauss(0, 1) for _ in range(n)]
y = [random.gauss(0, 1) for _ in range(n)]

# Per-point colors and sizes (Rust-level processing, zero Python loops)
colors = ['red' if xi > 0 else 'blue' for xi in x]
sizes = [max(10, abs(xi * yi * 50)) for xi, yi in zip(x, y)]

fig, ax = plt.subplots()
ax.scatter(x, y, c=colors, s=sizes, alpha=0.7)
ax.set_title('Scatter: Independent Colors/Sizes')
fig.savefig('scatter.png')
```

### Span Highlighting & Reference Lines

```python
from rsplotlib import pyplot as plt

fig, ax = plt.subplots()
x = list(range(-5, 6))
y = [xi**2 for xi in x]

# Plot data
ax.plot(x, y, 'b-', lw=2, label='y = x^2')

# Vertical span highlight
ax.axvspan(-1, 1, color='yellow', alpha=0.3, label='Minimum Region')

# Horizontal span highlight
ax.axhspan(0, 5, color='lightgreen', alpha=0.2)

# Horizontal/vertical reference lines
ax.axhline(y=0, color='gray', linestyle=':', linewidth=1)
ax.axvline(x=0, color='gray', linestyle=':', linewidth=1)

# Arbitrary-slope reference line (through the whole chart)
ax.axline((0, 0), (1, 1), color='red', linestyle='--', linewidth=1.5)

ax.legend()
fig.savefig('reference_lines.png')
```

### Arrowed Annotations

```python
from rsplotlib import pyplot as plt

fig, ax = plt.subplots()

# Plot curve
x = list(range(-10, 11))
y = [xi**3 - 3*xi for xi in x]
ax.plot(x, y, 'b-', lw=2)

# Add arrowed annotations
ax.annotate('Local Maximum', xy=(-1, 2), xytext=(-8, 500),
            fontsize=11, color='red',
            arrowprops={'arrowstyle': '->', 'arrowsize': 1.0})

ax.annotate('Local Minimum', xy=(1, -2), xytext=(3, -500),
            fontsize=11, color='blue',
            arrowprops={'arrowstyle': '->', 'arrowsize': 1.0})

ax.annotate('Origin', xy=(0, 0), xytext=(5, 200),
            fontsize=11, color='darkgreen')

ax.set_title('ax.annotate: Text with Arrow Annotations')
fig.savefig('annotations.png')
```

### Batch Horizontal/Vertical Lines (Rust-level implementation)

```python
from rsplotlib import pyplot as plt

fig, ax = plt.subplots()
ax.plot([0, 10], [0, 10], 'k-', lw=1)

# Draw horizontal lines in batch (Rust-level loop, zero Python overhead)
ax.hlines([2, 4, 6, 8], color='steelblue', linestyle='--', linewidth=1)

# Draw vertical lines in batch
ax.vlines([2, 4, 6, 8], color='darkorange', linestyle=':', linewidth=1)

ax.set_title('hlines / vlines: Batch Reference Lines (Rust-level)')
fig.savefig('hlines_vlines.png')
```

### Multiple Subplots

```python
from rsplotlib import pyplot as plt

# 2x2 grid
fig, axes = plt.subplots(2, 2)

# axes is a flat list
axes[0].plot([1, 2, 3], [1, 4, 9])
axes[0].set_title('Subplot 1')

axes[1].bar(['A', 'B', 'C'], [3, 7, 2])
axes[1].set_title('Subplot 2')

axes[2].scatter([1,2,3,4], [4,3,2,1], c=['red','green','blue','orange'])
axes[2].set_title('Subplot 3')

axes[3].hist([0.5, 1.2, 1.8, 2.1, 2.5, 3.0, 3.2, 3.8, 4.1, 4.5], bins=5)
axes[3].set_title('Subplot 4')

fig.savefig('subplots.png', dpi=200)
```

### Module-Level Interface (Without Explicit Axes)

```python
from rsplotlib import pyplot as plt

# Use plt.* directly
plt.figure()
plt.plot([1, 2, 3], [1, 2, 3], 'r-')
plt.axhline(y=2, color='gray', linestyle='--')
plt.axvspan(1, 2, color='yellow', alpha=0.3)
plt.title('Module-Level Interface')
plt.savefig('module_level.png')
```

---

## Feature Reference

### Plotting Functions

| Function         | Description                               | Module-Level | Axes Method |
| ---------------- | ----------------------------------------- | ------------ | ----------- |
| `plot()`         | Line plot                                 | Yes          | Yes         |
| `scatter()`      | Scatter plot (supports color/size arrays) | Yes          | Yes         |
| `bar()`          | Bar chart                                 | Yes          | Yes         |
| `barh()`         | Horizontal bar chart                      | Yes          | Yes         |
| `hist()`         | Histogram                                 | Yes          | Yes         |
| `pie()`          | Pie chart                                 | Yes          | Yes         |
| `boxplot()`      | Box plot                                  | Yes          | Yes         |
| `fill_between()` | Fill between lines                        | Yes          | Yes         |
| `errorbar()`     | Error bar plot                            | Yes          | Yes         |
| `stem()`         | Stem plot                                 | Yes          | Yes         |
| `step()`         | Step plot                                 | Yes          | Yes         |
| `imshow()`       | Image/heatmap                             | Yes          | Yes         |
| `stackplot()`    | Stacked area plot                         | Yes          | Yes         |
| `semilogx()`     | X-axis log-scale line plot                | Yes          | Yes         |
| `semilogy()`     | Y-axis log-scale line plot                | Yes          | Yes         |
| `loglog()`       | Log-log scale line plot                   | Yes          | Yes         |

### Annotation Elements

| Function     | Description                         | Module-Level | Axes Method |
| ------------ | ----------------------------------- | ------------ | ----------- |
| `axhline()`  | Horizontal reference line           | Yes          | Yes         |
| `axvline()`  | Vertical reference line             | Yes          | Yes         |
| `axhspan()`  | Horizontal span highlight           | Yes          | Yes         |
| `axvspan()`  | Vertical span highlight             | Yes          | Yes         |
| `axline()`   | Arbitrary-slope reference line      | Yes          | Yes         |
| `hlines()`   | Batch horizontal lines (Rust-level) | Yes          | Yes         |
| `vlines()`   | Batch vertical lines (Rust-level)   | Yes          | Yes         |
| `text()`     | Text annotation                     | Yes          | Yes         |
| `annotate()` | Arrowed text annotation             | Yes          | Yes         |

### Chart Configuration

| Function                               | Description                                      |
| -------------------------------------- | ------------------------------------------------ |
| `title()` / `ax.set_title()`           | Set chart title (`loc='left'/'center'/'right'`)  |
| `xlabel()` / `ax.set_xlabel()`         | Set X-axis label (`loc='left'/'center'/'right'`) |
| `ylabel()` / `ax.set_ylabel()`         | Set Y-axis label (`loc='top'/'center'/'bottom'`) |
| `grid()`                               | Show/hide grid lines                             |
| `legend()`                             | Show legend                                      |
| `xlim()` / `ylim()`                    | Set axis limits                                  |
| `xticks()` / `yticks()`                | Set tick positions and labels                    |
| `xscale()` / `yscale()`                | Set axis scale (`linear` / `log`)                |
| `margins()`                            | Set auto-scaling margins                         |
| `box()`                                | Set axes border display                          |
| `minorticks_on()` / `minorticks_off()` | Minor tick display control                       |

### Subplots & Layout

| Function                       | Description                 |
| ------------------------------ | --------------------------- |
| `subplots(nrows, ncols)`       | Create a subplot grid       |
| `subplot(nrows, ncols, index)` | Create a single subplot     |
| `twinx()` / `twiny()`          | Create twin axes            |
| `tight_layout()`               | Auto-adjust layout          |
| `fig.set_size(w, h)`           | Set figure pixel dimensions |

### Figure Control

| Function                      | Description                                |
| ----------------------------- | ------------------------------------------ |
| `figure()`                    | Create a new Figure object                 |
| `savefig(filename, dpi=None)` | Save to file, with custom DPI support      |
| `show()`                      | Display figure (saves to default location) |
| `gca()`                       | Get current Axes                           |
| `gcf()`                       | Get current Figure                         |
| `cla()`                       | Clear current Axes                         |
| `clf()`                       | Clear current Figure                       |
| `close()`                     | Close current Figure                       |

---

## API Reference

### Figure.savefig

Save the figure to a file.

```python
fig.savefig(filename, dpi=None)
```

**Parameters:**

- `filename` (str): Output file path. Supports `.png` and `.svg` extensions
- `dpi` (float, optional): Resolution (dots per inch). Defaults to Figure's DPI at creation time

**Example:**

```python
fig.savefig('plot.png')              # Default DPI
fig.savefig('plot_hd.png', dpi=150)  # Screen resolution
fig.savefig('plot_print.png', dpi=300)  # Print resolution
fig.savefig('plot.svg')              # Vector graphics
```

### Axes.scatter - Enhanced Version

Scatter plot with support for per-point independent colors and sizes.

```python
ax.scatter(x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0, **kwargs)
```

**Parameters:**

- `x`, `y` (list/array): Point coordinate data
- `s` (float or list/array): A single float value for all points, or an array for per-point sizes
- `c` (str or list[str]): A single color string, or an array of color strings
- `marker` (str): Marker shape. Supports `'o'`, `'s'`, `'^'`, `'v'`, `'D'`, `'*'`, `'+'`, `'x'`, `'<'`, `'>'`
- `label` (str): Legend label
- `alpha` (float): Transparency (0.0-1.0)
- `**kwargs`: Extra parameters. Supports `color` (as alias for `c`)

**Example:**

```python
# Single-color scatter
ax.scatter(x, y, c='red', s=50)

# Per-point independent colors
ax.scatter(x, y, c=['red', 'green', 'blue', ...], s=50)

# Per-point independent sizes
ax.scatter(x, y, s=[10, 20, 30, ...], c='blue')

# Simultaneously independent colors + sizes (Rust-level batch processing)
ax.scatter(x, y, c=colors, s=sizes, marker='D', alpha=0.8)
```

### Axes.axhspan / axvspan

Span highlighting fill.

```python
ax.axhspan(ymin, ymax, color=None, alpha=0.3)
ax.axvspan(xmin, xmax, color=None, alpha=0.3)
```

**Parameters:**

- `ymin`, `ymax` (float): Y-axis bounds for horizontal span (data coordinates)
- `xmin`, `xmax` (float): X-axis bounds for vertical span (data coordinates)
- `color` (str): Fill color, defaults to light gray-blue
- `alpha` (float): Transparency, default 0.3

### Axes.axline

Draw an arbitrary-slope reference line through two points, across the full chart.

```python
ax.axline(xy1, xy2, color=None, linestyle=None, linewidth=None)
```

**Parameters:**

- `xy1` (tuple): Starting coordinates `(x1, y1)`
- `xy2` (tuple): Ending coordinates `(x2, y2)`
- `color` (str): Line color
- `linestyle` (str): Line style. `'-'` (solid), `'--'` (dashed), `':'` (dotted), `'-.'` (dash-dot)
- `linewidth` (float): Line width

### Axes.annotate

Add arrowed text annotations.

```python
ax.annotate(text, xy, xytext=None, fontsize=12.0, color='black', arrowprops=None)
```

**Parameters:**

- `text` (str): Annotation text
- `xy` (tuple): Coordinates of the point being annotated `(x, y)`
- `xytext` (tuple, optional): Position to place the text. Defaults to `xy`
- `fontsize` (float): Font size, default 12.0
- `color` (str): Text color, default `'black'`
- `arrowprops` (dict, optional): Arrow properties dictionary. `None` (default)
  draws no arrow; when provided (even an empty dict) an arrow is drawn from the
  text-box edge to `xy`. Two modes, matching matplotlib:
  - **Simple** (no `arrowstyle` key): `width`, `headwidth`, `headlength` (points)
    and `shrink` (fraction) produce a filled arrow.
  - **Fancy** (`arrowstyle` given): `arrowstyle` (`'-'`, `'->'`, `'<-'`, `'<->'`,
    `'-|>'`, `'<|-'`, `'<|-|>'`, `'simple'`, `'fancy'`, `'wedge'`),
    `mutation_scale` (head size, defaults to font size), `shrinkA` / `shrinkB`
    (points), `linewidth` / `lw`, `color` / `ec`, `facecolor` / `fc`, `alpha`.

### hlines / vlines (Rust-level batch implementation)

Draw multiple horizontal or vertical lines. All internal looping is done at the Rust level, avoiding Python loop overhead.

```python
ax.hlines(y, color=None, linestyle=None, linewidth=None)
ax.vlines(x, color=None, linestyle=None, linewidth=None)
```

**Parameters:**

- `y` / `x` (list/array): Position list

---

## Performance Advantages

rsplotlib achieves performance optimization through a layered-down architecture strategy.

### Architecture Layers

```
+----------------------------------------------------+
| Python Layer (API Compatibility Layer)             |
|   * pyplot.py   (Matplotlib-compatible API)        |
|   * api.py      (Parameter normalization /         |
|                 alias mapping)                      |
|   * _patch_*    (Method patching / dynamic         |
|                 dispatch)                            |
+----------------------------------------------------+
| Rust Layer (High-Performance Core)                 |
|   * lib.rs      (Module registration / font        |
|                 system)                              |
|   * figure.rs   (Figure objects / render           |
|                 scheduling)                          |
|   * axes.rs     (Axes objects / data parsing)      |
|   * elements.rs (Plot element data structures)     |
|   * axes_render_elements.rs (plotters render)      |
|   * pyfuncs.rs  (Module-level function             |
|                 exposure)                            |
+----------------------------------------------------+
```

### Performance-Critical Paths Offloaded to Rust

| Feature                     | Traditional Implementation                     | rsplotlib Implementation              |
| --------------------------- | ---------------------------------------------- | ------------------------------------- |
| `scatter(c=colors)`         | Python iterates over every point               | Rust `ScatterMulti` unified rendering |
| `scatter(s=sizes)`          | Python iterates over every point               | Rust unified size array processing    |
| `hlines([y1, y2, y3, ...])` | Python `for` loop calling `axhline` repeatedly | Single Rust call, batch processing    |
| `vlines([x1, x2, x3, ...])` | Python `for` loop calling `axvline` repeatedly | Single Rust call, batch processing    |
| `savefig(dpi=300)`          | No DPI support / Python scaling                | Rust writes PNG DPI metadata directly |

### Why a Rust + Python Hybrid Architecture?

1. **Development Speed**: Rapid iteration on API design and parameter validation at the Python layer
2. **Execution Performance**: Rust handles rendering, looping, and memory-intensive computations
3. **Memory Safety**: Rust guarantees no data races at compile time
4. **Zero Extra Dependencies**: No native Matplotlib installation required

---

## Project Structure

```
rsplotlib/
+-- python/                          # Python wrapper layer
|   +-- rsplotlib/
|       +-- __init__.py              # Package entry & exports
|       +-- api.py                   # Module-level API function definitions
|       +-- pyplot.py                # Matplotlib-compatible pyplot interface
|       |                            # (with docstrings for IDE hover)
|       +-- pylab.py                 # pylab-style interface, provides mpl.rcParams
|       +-- style.py                 # plt.style style management
|       +-- _rcparams.py             # rcParams configuration management
|       +-- _font_resolver.py        # System font path resolution
|       +-- _figure_defaults.py      # Figure default configuration
|       +-- gridspec.py              # Grid layout management
|       +-- ticker.py                # Tick locator (AutoLocator, MaxNLocator)
|
+-- src/                             # Rust core implementation
|   +-- lib.rs                       # Library entry point, Python module registration
|   +-- figure.rs                    # Figure class (savefig, DPI management)
|   +-- axes.rs                      # Axes class (all plotting methods)
|   +-- axes_render_elements.rs      # Element rendering engine (plotters-based)
|   +-- axes_bounds.rs               # Coordinate boundary calculations
|   +-- axes_title.rs                # Title rendering
|   +-- axes_legend.rs               # Legend rendering
|   +-- axes_grid.rs                 # Grid line rendering
|   +-- axes_mesh.rs                 # Axis tick marks
|   +-- axis.rs                      # Axis data structure
|   +-- elements.rs                  # Plot element enum (Line, Scatter, Bar, ...)
|   +-- pyfuncs.rs                   # Module-level functions exposed to Python
|   +-- colors.rs                    # Color parsing (named, RGB, HSL)
|   +-- colormap.rs                  # Color mapping
|   +-- marker.rs                    # Marker shape rendering
|   +-- text_utils.rs                # Text utility functions
|
+-- Cargo.toml                       # Rust dependencies (PyO3, plotters, png)
+-- pyproject.toml                   # Python package configuration (maturin)
+-- build_wheel.sh                   # Wheel package build script
+-- README.md                        # English documentation (this file)
+-- README_zh.md                     # Chinese documentation
```

---

## Development & Contributing

### Local Development Environment

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install maturin
pip install maturin

# 3. Development-mode build (compiles Rust, installs to current Python env)
maturin develop --release

# 4. Run interactive tests
python3 -c "
from rsplotlib import pyplot as plt
fig, ax = plt.subplots()
ax.plot([1,2,3], [1,4,9])
fig.savefig('/tmp/test.png')
print('OK')
"
```

### Development Workflow

1. **Understand Requirements**: Analyze which Matplotlib functionality needs implementation
2. **Implement at Rust Layer**: Add core logic in `src/`
   - Add new plot elements: Modify `elements.rs`
   - Add plotting methods: Modify `axes.rs`
   - Add rendering logic: Modify `axes_render_elements.rs`
   - Expose module-level functions: Modify `pyfuncs.rs`
   - Register: Modify `lib.rs`
3. **Wrap at Python Layer**: Add API in `python/rsplotlib/`
   - Add docstrings (for IDE hover display)
   - Handle parameter aliases (e.g., `lw` -> `linewidth`)
   - Route array parameters to Rust batch methods
4. **Compile & Test**: `maturin develop --release`
5. **Verify Results**: Generate images for visual inspection

### Adding Documentation to the Python Layer (for IDE Hover)

Functions in the Python wrapper include detailed docstrings so that when users hover over them in IDEs (VS Code, PyCharm, etc.), they automatically see descriptions and parameter lists.

```python
def axhspan(ymin, ymax, **kwargs):
    """Draw a horizontal span (colored region).

    Usage:
        plt.axhspan(0, 1, color='yellow', alpha=0.3)

    Args:
        ymin: Lower y-axis bound
        ymax: Upper y-axis bound
        color: Fill color (defaults to blue-gray)
        alpha: Transparency (0.0-1.0, default 0.3)
    """
    ...
```

### Contributing Guidelines

PRs are welcome! Please include:

- A clear description of the feature or bug fix
- Generated example images, when applicable
- Make sure to update both `README.md` and `README_zh.md`

### Known Limitations

- 3D plotting is not supported in the current version
- Animated/interactive charts are not supported
- `contour` / `violinplot` / `hexbin` are placeholder implementations

---

## Font Configuration

rsplotlib supports custom fonts via `mpl.rcParams`, as well as direct font file registration.

### Auto-Detected System Fonts

| Platform    | Common Fonts                                                               |
| ----------- | -------------------------------------------------------------------------- |
| **macOS**   | Arial, Helvetica, PingFang SC, STHeiti, Hiragino Sans GB, Arial Unicode MS |
| **Linux**   | DejaVu Sans, Liberation Sans, Noto Sans CJK SC, WenQuanYi Micro Hei        |
| **Windows** | Microsoft YaHei, SimHei, SimSun, Arial                                     |

### Custom Font Configuration

```python
from rsplotlib.pylab import mpl
from rsplotlib import pyplot as plt

# Set sans-serif font family (priority order)
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Microsoft YaHei', 'DejaVu Sans']

# Set default font size
mpl.rcParams['font.size'] = 12
```

### Direct Font File Registration

```python
from rsplotlib import rsplotlib as _rs

# Register any .ttf/.otf/.ttc file as sans-serif
_rs.register_sans_serif_font('/path/to/your/custom-font.ttf')
```

---

## License

MIT License - See the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- [**PyO3**](https://github.com/PyO3/pyo3) - Rust <-> Python bindings library, version 0.29
- [**plotters**](https://github.com/plotters-rs/plotters) - Rust plotting library, provides underlying rendering capability
- [**Matplotlib**](https://matplotlib.org/) - Python plotting ecosystem standard, provides API design reference
- [**maturin**](https://github.com/PyO3/maturin) - Rust Python package build and publication tool

---

## Related Links

- **GitHub**: https://github.com/YJ-Niu/rsplotlib

---

_Last updated: 2026-07-03 / Version v0.1.9_
