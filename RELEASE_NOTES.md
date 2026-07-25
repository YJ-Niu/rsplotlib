# Release Notes

All notable changes to **rsplotlib** are documented here. The format is loosely
based on [Keep a Changelog](https://keepachangelog.com/), and the project follows
[Semantic Versioning](https://semver.org/).

> 中文版发行说明见 [RELEASE_NOTES_zh.md](RELEASE_NOTES_zh.md)。

---

## v0.3.5 — 2026-07-25

Font rendering and text layout improvements.

### Fixed

- **Default font size**: Increased default text font size by 10% for better readability.
- **Text alignment defaults**: Fixed mismatch between text alignment default values and Python interface parameters.

### Maintenance

- Code cleanup and dependency optimization.

---

## v0.3.4 — 2026-07-24

Legend and build improvements.

### Fixed

- **Legend ellipsis**: Fixed legend text ellipsis handling for long labels.
- **Legend layout**: Optimized legend positioning and spacing.

### Maintenance

- Optimized build configuration for better compilation performance.

---

## v0.3.3 — 2026-07-23

Core fixes and refactoring.

### Fixed

- **Subplot layout**: Fixed vertical alignment issues for special layout subplots.
- **Suptitle rendering**: Fixed figure `suptitle` rendering logic and position calculation.
- **Touchstone parsing**: Fixed touchstone file parsing issues.

### Changed

- **Dependency refactoring**: Removed `rsnumpy` dependency, unified array object handling.
- **Code cleanup**: Removed redundant test files and deprecated code.

---

## v0.3.2 — 2026-07-22

Testing and interpolation improvements.

### Added

- **Scipy interpolation simulation**: Added scipy-compatible interpolation functions.

### Fixed

- **Test file fixes**: Fixed various test file issues and refactored test plotting subplot layout logic.

---

## v0.3.1 — 2026-07-21

Layout and CI improvements.

### Changed

- **Subplot spacing**: Adjusted subplot base spacing values for better visual layout.
- **CI workflow**: Adjusted installation step order in CI workflows.

---

## v0.3.0 — 2026-07-20

Major API and architecture improvements with significant refactoring.

### Added

- **Extended test coverage**: Added comprehensive test files and completed library structure.
- **Float rounding handling**: Added float rounding handling for better display precision.

### Changed

- **Core refactoring**: Major refactoring of core modules including colors and pyplot.
- **Code formatting**: Batch optimization of code formatting and dependency replacement.

### Fixed

- **Chart-related issues**: Fixed multiple chart rendering issues and refactored dependencies.

---

## v0.2.9 — 2026-07-12

Rendering performance and CI maintenance. No API changes.

### Performance

- **Multi-threaded image rendering**: `imshow` row rendering and image
  down-sampling now run across multiple threads (bounded by available cores).

### Maintenance

- Adjusted the CI test matrix and aligned the `clippy` lint configuration
  across workflows.

---

## v0.2.8 — 2026-07-11

Performance-focused release. Large-dataset rendering paths were rewritten to avoid
materializing millions of Python objects. All optimizations are automatic and
require no API changes.

### Performance

- **Histogram zero-copy path**: `hist()` now feeds pure numeric buffers straight
  into Rust via the buffer protocol, removing million-scale Python object
  materialization for large inputs.
- **Boxplot zero-copy path**: `boxplot()` numeric arrays are pushed down to Rust
  the same way, eliminating per-value Python overhead.
- **Line decimation**: line plots automatically down-sample with a min/max
  (M4-style) algorithm when the point count greatly exceeds the pixel columns,
  preserving visual shape while cutting render time.
- **Glyph cache**: rendered glyph coverage is cached by `(font, char, size)`,
  speeding up text-heavy figures.

---

## v0.2.7 — 2026-07-11

- Added several additional matplotlib-compatible features and API refinements
  across the plotting surface.

---

## v0.2.6 — 2026-07-09

Colorbar and colormap release (includes work tagged internally as 0.2.4 / 0.2.5).

### Added

- **Colorbar**: `plt.colorbar()` and `fig.colorbar()` backed by a Rust renderer,
  with support for `location`, `orientation`, `shrink`, `aspect`, `pad`,
  `fraction`, `label`, `extend`, `ticks`, and `format`.
- **Multiple colormaps**: a large built-in set including `viridis`, `plasma`,
  `inferno`, `magma`, `cividis`, `jet`, `coolwarm`, `RdBu`, `Blues`, `Greens`,
  `Reds`, `hot`, `cool`, `gray`, `terrain`, `twilight`, and many more. Any name
  can be reversed with a `_r` suffix (e.g. `viridis_r`).
- **Logarithmic color normalization**: `LogNorm` / `Normalize` (from
  `rsplotlib.colors`) usable via the `norm=` argument of `imshow`.
- **Multi-format / multi-curve plotting**: broader `plot()` support for multiple
  curves and matplotlib-style format strings.

### Changed

- Adjusted `annotate` default font size and legend layout.
- Tuned colorbar thickness and tick length; removed a redundant border draw.

---

## v0.2.2 – v0.2.3 — 2026-07-07

Text rendering and layout release.

### Added

- **Mathtext**: lightweight LaTeX-style math rendering for `$...$` expressions,
  supporting superscripts/subscripts, `\frac`, `\sqrt[n]{}`, Greek letters,
  accents, and font-style commands. Active in titles, axis labels, `text`,
  `annotate`, legend labels, and bar labels.
- **Full arrow annotations**: `annotate` gained complete arrow-style support
  (simple and fancy `arrowstyle` modes).
- **Spanning subplots & categorical axes**: `GridSpec` slicing such as
  `gs[a:b, c:d]` lets a subplot span multiple grid cells; bar charts accept
  string categories.
- **Scatter stroke**: `scatter` accepts `edgecolors` / `edgecolor` and
  `linewidths` / `linewidth`.
- **`data=` argument** for `scatter`, matplotlib style — pass a dict and refer to
  columns by string key.
- New `axes` API surface and improved `add_subplot` compatibility.

### Fixed

- Fixed overlapping x-axis tick labels with adaptive tick thinning and automatic
  subplot spacing.
- Fixed square-root rendering and adjusted legend text offset.

---

## v0.2.0 – v0.2.1 — 2026-07-06

Image release.

### Added

- **`imshow` rework**: RGB/RGBA image input, alpha, `origin`, and `interpolation`
  (`none`/`nearest` block scaling vs. smooth modes).
- **`imread` / `imsave`**: read and write PNG/JPEG images.
- Image interpolation support and updated dependencies; refactored `rcParams`
  and `subplots`.

---

## v0.1.9 — 2026-07-03

First PyPI release with prebuilt wheels.

### Added

- Label/title position support (`loc='left'/'center'/'right'`, etc.).
- Python 3.10–3.14 support; prebuilt wheels for Linux (x86_64/aarch64),
  macOS (universal2), and Windows (x64).

### Fixed

- `grid()` parameter compatibility.
- Linux font fallback (DejaVu / Liberation / Noto CJK) so text renders on
  runners without preinstalled fonts.

---

## Earlier (≤ v0.1.8)

Foundational work: the Rust + PyO3 rendering core (plotters backend), the
matplotlib-compatible `pyplot` API, PNG/SVG/JPG output with DPI metadata,
per-point scatter colors/sizes, batch `hlines`/`vlines`, equal-width line
rendering, and the cross-platform font resolver.

---

## Notes & Known Limitations

- `imshow(extent=...)` is accepted for signature compatibility but currently
  ignored by the renderer.
- `colorbar()` accepts many matplotlib kwargs, but only `location`,
  `orientation`, `shrink`, `aspect`, `pad`, `fraction`, `label`, `extend`,
  `ticks`, and `format` take effect.
- `data=` is currently supported by `scatter` only (not `plot`).
- 3D plotting and animated/interactive charts are not supported;
  `contour` / `violinplot` / `hexbin` are placeholders.
- Features such as `explode`, `edgecolor`, `data=`, and mathtext are exposed
  through `rsplotlib.pyplot`; the legacy top-level `rsplotlib.*` functions may
  use different defaults.
