# rsplotlib

A high-performance Python plotting library implemented in Rust, designed to be compatible with Matplotlib's API while leveraging Rust's performance and portability.

## Features

- **Matplotlib Compatible API**: Supports common plotting functions like `plot`, `scatter`, `bar`, `hist`, `pie`, etc.
- **High Performance**: Core rendering engine written in Rust using the plotters library
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **Custom Font Support**: Supports font customization via `mpl.rcParams`
- **Multiple Backends**: Supports SVG and PNG output

## Installation

### Prerequisites

- Python 3.9+
- Rust 1.70+ (for building from source)
- uv (recommended for Python package management)

### Building from Source

```bash
# Create a virtual environment
uv venv --python 3.13

# Activate the virtual environment
source .venv/bin/activate

# Build the Rust extension and install the package
./build_wheel.sh

# Install dependencies
uv pip install matplotlib numpy
```

## Quick Start

```python
from rsplotlib import pyplot as plt
from rsplotlib.pylab import mpl

# Configure font (optional)
mpl.rcParams['font.sans-serif'] = ['Arial']

# Create a figure
fig, ax = plt.subplots()

# Plot data
ax.plot([0, 1, 2, 3], [1, 4, 2, 3], label='Line')
ax.scatter([0, 1, 2, 3], [2, 3, 1, 4], color='red', label='Points')

# Add labels and title
ax.set_xlabel('X Axis')
ax.set_ylabel('Y Axis')
ax.set_title('Sample Plot')
ax.legend()

# Save the figure
fig.savefig('output.png')
```

## Supported Functions

### Plotting Functions

- `plot()` - Line plot
- `scatter()` - Scatter plot
- `bar()` - Bar chart
- `barh()` - Horizontal bar chart
- `hist()` - Histogram
- `pie()` - Pie chart
- `boxplot()` - Box plot
- `fill_between()` - Fill between lines
- `errorbar()` - Error bar plot
- `stem()` - Stem plot
- `step()` - Step plot
- `imshow()` - Image display

### Text Functions

- `text()` - Add text
- `title()` - Add title
- `xlabel()` - Add X axis label
- `ylabel()` - Add Y axis label

### Configuration Functions

- `grid()` - Toggle grid
- `legend()` - Show legend
- `xlim()` / `ylim()` - Set axis limits
- `xticks()` / `yticks()` - Set tick positions
- `xscale()` / `yscale()` - Set axis scale (linear/log)
- `use()` - Set backend

### Subplot Functions

- `subplots()` - Create subplot grid
- `subplot()` - Create single subplot
- `twinx()` / `twiny()` - Create twin axes
- `tight_layout()` - Adjust layout

## Font Configuration

rsplotlib supports font customization through `mpl.rcParams`:

```python
from rsplotlib.pylab import mpl

# Set font family
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Microsoft YaHei', 'DejaVu Sans']

# Set font size
mpl.rcParams['font.size'] = 12
```

### Supported Fonts

- **macOS**: Arial, Helvetica, PingFang SC, STHeiti, Hiragino Sans GB
- **Linux**: DejaVu Sans, Liberation Sans, Noto Sans CJK SC, WenQuanYi Micro Hei
- **Windows**: Microsoft YaHei, SimHei, SimSun

## Project Structure

```
rsplotlib/
├── python/
│   └── rsplotlib/          # Python wrapper implementation
│       ├── __init__.py      # Package exports
│       ├── api.py           # Public API definitions
│       ├── pyplot.py        # pyplot module (Matplotlib compatible)
│       ├── pylab.py         # pylab module with mpl.rcParams
│       ├── _rcparams.py     # rcParams configuration management
│       └── _font_resolver.py # Font path resolution
├── src/                     # Rust implementation
│   ├── lib.rs              # Rust library entry
│   ├── pyfuncs.rs          # Python-exposed functions
│   ├── figure.rs           # Figure implementation
│   ├── axes.rs             # Axes implementation
│   └── axes_render_elements.rs # Rendering elements
├── Cargo.toml              # Rust dependencies
├── pyproject.toml          # Python package configuration
└── build_wheel.sh          # Build script
```

## Development Workflow

1. **Fix**: Identify and fix issues based on test cases
2. **Refactor**: Clean up code, prioritize changes in `python/rsplotlib/`
3. **Plot**: Generate comparison images using `main.py` or `python/examples.py`
4. **Compare**: Compare generated PNGs with Matplotlib reference images
5. **Iterate**: Repeat the cycle based on differences

## Contributing

When submitting a PR, please include:

- Generated example images
- Description of the issues fixed or improvements made

## License

MIT License

## Acknowledgments

- [plotters](https://github.com/plotters-rs/plotters) - Rust plotting library
- [Matplotlib](https://matplotlib.org/) - Python plotting library for API reference
