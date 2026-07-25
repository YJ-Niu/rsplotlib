# 发行说明

本文件记录 **rsplotlib** 的所有重要变更。格式大致参考
[Keep a Changelog](https://keepachangelog.com/)，版本遵循
[语义化版本](https://semver.org/)。

> English release notes: [RELEASE_NOTES.md](RELEASE_NOTES.md)。

---

## v0.3.5 — 2026-07-25

字体渲染和文本布局改进。

### 修复

- **默认字体大小**：默认文字字号放大 10%，提升可读性。
- **文本对齐默认值**：修复文本对齐默认值和 Python 接口参数不匹配问题。

### 维护

- 代码清理和依赖优化。

---

## v0.3.4 — 2026-07-24

图例和构建改进。

### 修复

- **图例省略号**：修复长标签的图例文本省略号处理。
- **图例布局**：优化图例定位和间距。

### 维护

- 优化构建配置，提升编译性能。

---

## v0.3.3 — 2026-07-23

核心修复和重构。

### 修复

- **子图布局**：修复特殊布局子图的垂直对齐问题。
- **Suptitle 渲染**：修复图表 `suptitle` 渲染逻辑和位置计算。
- **Touchstone 解析**：修复 touchstone 文件解析问题。

### 变更

- **依赖重构**：移除 `rsnumpy` 依赖，统一数组对象处理。
- **代码清理**：删除冗余测试文件和废弃代码。

---

## v0.3.2 — 2026-07-22

测试和插值改进。

### 新增

- **Scipy 插值模拟**：添加 scipy 兼容的插值函数。

### 修复

- **测试文件修复**：修复各种测试文件问题，重构测试绘图的子图布局逻辑。

---

## v0.3.1 — 2026-07-21

布局和 CI 改进。

### 变更

- **子图间距**：调整子图基础间距值，优化视觉布局。
- **CI 工作流**：调整 CI 工作流中的安装步骤顺序。

---

## v0.3.0 — 2026-07-20

主要 API 和架构改进，包含重大重构。

### 新增

- **扩展测试覆盖**：添加全面的测试文件并完善库结构。
- **浮点数舍入处理**：添加浮点数舍入处理，提升显示精度。

### 变更

- **核心重构**：对 colors 和 pyplot 等核心模块进行重大重构。
- **代码格式化**：批量优化代码格式和依赖替换。

### 修复

- **图表相关问题**：修复多个图表渲染问题并重构依赖。

---

## v0.2.9 — 2026-07-12

渲染性能优化与 CI 维护，无 API 变更。

### 性能

- **图像多线程渲染**：`imshow` 的按行渲染与图像下采样现在跨多线程并行
  （线程数受可用核心数限制）。

### 维护

- 调整 CI 测试矩阵，并统一各 workflow 的 `clippy` 检查配置。

---

## v0.2.8 — 2026-07-11

性能优化专版。大数据渲染路径经过重写，避免在 Python 侧物化数百万个对象。
所有优化自动生效，无需改动 API。

### 性能

- **直方图零拷贝下沉**：`hist()` 现在通过缓冲协议将纯数值数组直接传给 Rust，
  大数据下不再有百万级 Python 对象物化开销。
- **箱线图零拷贝下沉**：`boxplot()` 的数值数组以同样方式下沉到 Rust，
  消除逐值的 Python 开销。
- **折线抽稀**：当点数远超像素列数时，折线自动使用 min/max（M4 风格）算法
  下采样，在保留视觉形态的同时大幅降低渲染耗时。
- **字形缓存**：按 `(字体, 字符, 字号)` 缓存字形覆盖率，加速文本密集的图像。

---

## v0.2.7 — 2026-07-11

- 新增多项 matplotlib 兼容特性，并对绘图接口做了若干细节完善。

---

## v0.2.6 — 2026-07-09

颜色条与颜色映射版本（含内部标记为 0.2.4 / 0.2.5 的工作）。

### 新增

- **颜色条 Colorbar**：`plt.colorbar()` 与 `fig.colorbar()`，由 Rust 渲染，
  支持 `location`、`orientation`、`shrink`、`aspect`、`pad`、`fraction`、
  `label`、`extend`、`ticks`、`format`。
- **多种颜色映射**：内置大量 colormap，包括 `viridis`、`plasma`、`inferno`、
  `magma`、`cividis`、`jet`、`coolwarm`、`RdBu`、`Blues`、`Greens`、`Reds`、
  `hot`、`cool`、`gray`、`terrain`、`twilight` 等。任意名称加 `_r` 后缀即可反转
  （如 `viridis_r`）。
- **对数颜色归一化**：`LogNorm` / `Normalize`（来自 `rsplotlib.colors`），
  通过 `imshow` 的 `norm=` 参数使用。
- **多格式 / 多曲线绘图**：`plot()` 支持更完善的多曲线与 matplotlib 风格格式串。

### 变更

- 调整 `annotate` 默认字号与图例布局。
- 微调颜色条厚度与刻度长度，移除冗余边框绘制。

---

## v0.2.2 – v0.2.3 — 2026-07-07

文本渲染与布局版本。

### 新增

- **数学公式 Mathtext**：轻量 LaTeX 风格 `$...$` 渲染，支持上下标、`\frac`、
  `\sqrt[n]{}`、希腊字母、重音与字体样式命令。在标题、轴标签、`text`、
  `annotate`、图例标签与柱状图标签中生效。
- **完整箭头标注**：`annotate` 支持完整的箭头样式（simple 与 fancy 的
  `arrowstyle` 两种模式）。
- **跨格子图与分类坐标轴**：`GridSpec` 切片（如 `gs[a:b, c:d]`）可让子图跨越多个
  网格单元；柱状图可直接接受字符串类别。
- **散点描边**：`scatter` 支持 `edgecolors` / `edgecolor` 与
  `linewidths` / `linewidth`。
- **`data=` 参数**（`scatter`，matplotlib 风格）——传入 dict 并用字符串键引用列。
- 新增 `axes` API，改进 `add_subplot` 兼容性。

### 修复

- 修复 X 轴刻度标签重叠：新增自适应刻度稀释与自动子图间距。
- 修复根号渲染，调整图例文本偏移。

---

## v0.2.0 – v0.2.1 — 2026-07-06

图像版本。

### 新增

- **`imshow` 重构**：支持 RGB/RGBA 图像输入、透明度、`origin` 与 `interpolation`
  （`none`/`nearest` 块状缩放，以及平滑模式）。
- **`imread` / `imsave`**：读写 PNG/JPEG 图像。
- 图像插值支持，更新依赖；重构 `rcParams` 与 `subplots`。

---

## v0.1.9 — 2026-07-03

首个带预编译 wheel 的 PyPI 发布。

### 新增

- 标签/标题定位支持（`loc='left'/'center'/'right'` 等）。
- 支持 Python 3.10–3.14；提供 Linux（x86_64/aarch64）、macOS（universal2）、
  Windows（x64）预编译 wheel。

### 修复

- `grid()` 参数兼容性。
- Linux 字体回退（DejaVu / Liberation / Noto CJK），使未预装字体的机器也能正常渲染文本。

---

## 更早（≤ v0.1.8）

基础工作：Rust + PyO3 渲染核心（plotters 后端）、matplotlib 兼容的 `pyplot`
API、带 DPI 元数据的 PNG/SVG/JPG 输出、逐点散点颜色/大小、批量 `hlines`/`vlines`、
等宽折线渲染，以及跨平台字体解析器。

---

## 说明与已知限制

- `imshow(extent=...)` 为保持签名兼容而接受，但渲染器当前忽略该参数。
- `colorbar()` 接受较多 matplotlib 关键字，但仅 `location`、`orientation`、
  `shrink`、`aspect`、`pad`、`fraction`、`label`、`extend`、`ticks`、`format`
  真正生效。
- `data=` 目前仅 `scatter` 支持（`plot` 不支持）。
- 暂不支持 3D 绘图与动画/交互图；`contour` / `violinplot` / `hexbin` 为占位实现。
- `explode`、`edgecolor`、`data=`、mathtext 等特性通过 `rsplotlib.pyplot` 暴露；
  旧的顶层 `rsplotlib.*` 函数可能使用不同的默认值。
