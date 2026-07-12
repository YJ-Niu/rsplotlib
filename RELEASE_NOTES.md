# Release Notes

All notable changes to **rsplotlib** are documented here. The format is loosely
based on [Keep a Changelog](https://keepachangelog.com/), and the project follows
[Semantic Versioning](https://semver.org/).

> 中文版发行说明见 [RELEASE_NOTES_zh.md](RELEASE_NOTES_zh.md)。

---

## v0.2.8 — 2026-07-11

Performance-focused release. Large-dataset rendering paths were rewritten to avoid
materializing millions of Python objects, and image rendering was parallelized.
All optimizations are automatic and require no API changes.

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
- **Multi-threaded image rendering**: `imshow` row rendering and image
  down-sampling now run across multiple threads (bounded by available cores).

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
