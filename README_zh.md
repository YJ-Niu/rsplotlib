# rsplotlib

> 一个由 Rust 强力驱动的高性能 Python 绘图库，提供 Matplotlib 兼容 API

[![Python](https://img.shields.io/badge/Python-3.10%2B-blue)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/Rust-2024-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![PyO3](https://img.shields.io/badge/PyO3-0.29-2c2d72)](https://pyo3.rs/)
[![plotters](https://img.shields.io/badge/plotters-0.3-7d5cff)](https://github.com/plotters-rs/plotters)

---

## 目录

- [项目简介](#项目简介)
- [核心特性](#核心特性)
- [安装指南](#安装指南)
- [快速入门](#快速入门)
- [功能清单](#功能清单)
- [API 参考](#api-参考)
- [性能优势](#性能优势)
- [项目结构](#项目结构)
- [开发与贡献](#开发与贡献)
- [许可证](#许可证)

---

## 项目简介

**rsplotlib** 是一个跨语言的 Python 绘图库，核心渲染引擎完全使用 Rust 编写，通过 PyO3 提供与 Matplotlib 兼容的 Python API。项目的目标是在保持与现有 Matplotlib 代码最大兼容性的同时，利用 Rust 的内存安全和零成本抽象带来显著的性能提升。

### 设计理念

- **API 优先**：提供与 Matplotlib 高度一致的接口，降低迁移成本
- **性能为王**：将性能关键路径（渲染、批量操作）下沉到 Rust 层
- **零额外依赖**：不依赖原生 Matplotlib 的安装，仅需 Python 解释器即可运行
- **跨平台一致**：在 macOS、Linux、Windows 上提供相同的渲染质量

---

## 核心特性

### 丰富的图表类型

- **基础图表**：折线图、散点图、柱状图、水平柱状图
- **统计图表**：直方图、箱线图、饼图、误差棒图、茎叶图、阶梯图
- **高级图表**：堆叠面积图、热力图/图像显示、填充区域图

### 辅助元素

- **参考线**：水平参考线 (`axhline`)、垂直参考线 (`axvline`)
- **区间高亮**：水平区间填充 (`axhspan`)、垂直区间填充 (`axvspan`)
- **任意斜率参考线**：`axline` — 通过两点绘制贯穿全图的任意角度参考线
- **批量线**：`hlines` / `vlines` — Rust 层批量实现，避免 Python 级 for 循环
- **文本标注**：`text`、`title`、带箭头的 `annotate`（支持 `arrowprops`）

### 坐标与布局

- 多子图支持：`subplots()`、`subplot()`、网格布局
- 双坐标轴：`twinx()` / `twiny()`
- 对数坐标：`semilogx()`、`semilogy()`、`loglog()`
- 网格显示、图例、坐标轴范围、刻度自定义

### 输出与分辨率

- **PNG**：位图输出，支持 DPI 控制
- **SVG**：矢量图输出，可无损缩放
- **自定义 DPI**：`savefig(filename, dpi=300)` 支持任意分辨率输出
- **像素尺寸 DPI**：PNG 元数据中写入 DPI 信息，方便出版使用

### 散点图增强

- **每点独立颜色**：`ax.scatter(x, y, c=['red', 'blue', 'green'])`
- **每点独立大小**：`ax.scatter(x, y, s=[10, 20, 30, 40])`
- **颜色+大小同时独立**：Rust 层实现，零 Python 循环开销

### 样式管理

- **`plt.style` 模块**：兼容 Matplotlib 的样式接口
- **`plt.style.use()`**：应用预设样式
- **`plt.style.available`**：查询可用样式
- **`plt.rcParams`**：全局参数配置（字体族、字号等）

### 字体与国际化

- **跨平台字体解析**：自动查找系统可用字体
- **CJK 支持**：内置中文、日文、韩文字体自动检测
- **自定义字体注册**：`register_sans_serif_font(path)` 注册任意字体文件

---

## 安装指南

### 前置依赖

| 依赖    | 版本要求 | 说明                   |
| ------- | -------- | ---------------------- |
| Python  | 3.10+    | CPython 实现           |
| Rust    | 1.70+    | 从源码构建时需要       |
| maturin | 1.13+    | Rust-Python 包构建工具 |

### 方法一：从 PyPI 安装（推荐）

已为 Linux（x86_64/aarch64）、macOS（universal2）、Windows（x64）在 Python 3.10-3.14 上发布预编译 wheel，无需 Rust 工具链：

```bash
pip install rsplotlib
```

### 方法二：使用 maturin 从源码构建（推荐开发者）

```bash
# 1. 克隆项目
git clone https://github.com/YJ-Niu/rsplotlib.git
cd rsplotlib

# 2. 构建并安装（自动编译 Rust 并安装到当前 Python 环境）
pip install maturin
maturin develop --release
```

### 方法三：构建 wheel 包

```bash
# 使用项目提供的构建脚本（macOS/Linux 直接运行；Windows 请在 Git Bash 或 WSL 中运行）
./build_wheel.sh

# 安装生成的 wheel
pip install target/wheels/rsplotlib-*.whl
```

### 方法四：仅编译 Rust 扩展（调试用）

```bash
# 编译 Rust cdylib
cargo build --release

# 将编译产物复制为正确的 Python 扩展模块名称
# macOS:
cp target/release/librsplotlib.dylib python/rsplotlib/rsplotlib.cpython-39-darwin.so

# Linux:
# cp target/release/librsplotlib.so python/rsplotlib/rsplotlib.cpython-39-x86_64-linux-gnu.so

# 从源码目录使用
export PYTHONPATH="$PWD/python:$PYTHONPATH"
```

### 验证安装

```python
>>> import rsplotlib.pyplot as plt
>>> fig, ax = plt.subplots()
>>> ax.plot([1, 2, 3], [1, 4, 9])
>>> fig.savefig('test.png')
# ✅ 生成 test.png 即安装成功
```

---

## 快速入门

### 基础使用：折线图

```python
from rsplotlib import pyplot as plt
from rsplotlib.pylab import mpl

# 可选：配置中文字体
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Microsoft YaHei', 'Arial']

# 创建 Figure 和 Axes
fig, ax = plt.subplots(figsize=(8, 6))

# 绘制数据
x = [0, 1, 2, 3, 4, 5]
y1 = [1, 2, 4, 8, 16, 32]
y2 = [32, 16, 8, 4, 2, 1]

ax.plot(x, y1, label='指数增长', lw=2.0)
ax.plot(x, y2, label='指数衰减', lw=2.0, linestyle='--')

# 装饰图表
ax.set_title('基础折线图示例')
ax.set_xlabel('时间')
ax.set_ylabel('数值')
ax.legend()
ax.grid(True)

# 保存为高分辨率 PNG
fig.savefig('line_plot.png', dpi=300)
```

### 散点图：每点独立颜色和大小

```python
import random
from rsplotlib import pyplot as plt

# 生成随机数据
random.seed(42)
n = 50
x = [random.gauss(0, 1) for _ in range(n)]
y = [random.gauss(0, 1) for _ in range(n)]

# 每点独立颜色和大小（Rust 层处理，Python 层零循环）
colors = ['red' if xi > 0 else 'blue' for xi in x]
sizes = [max(10, abs(xi * yi * 50)) for xi, yi in zip(x, y)]

fig, ax = plt.subplots()
ax.scatter(x, y, c=colors, s=sizes, alpha=0.7)
ax.set_title('散点图：颜色/大小独立')
fig.savefig('scatter.png')
```

### 区间高亮与参考线

```python
from rsplotlib import pyplot as plt

fig, ax = plt.subplots()
x = list(range(-5, 6))
y = [xi**2 for xi in x]

# 绘制数据
ax.plot(x, y, 'b-', lw=2, label='y = x²')

# 垂直区间高亮
ax.axvspan(-1, 1, color='yellow', alpha=0.3, label='最小值区域')

# 水平区间高亮
ax.axhspan(0, 5, color='lightgreen', alpha=0.2)

# 水平/垂直参考线
ax.axhline(y=0, color='gray', linestyle=':', linewidth=1)
ax.axvline(x=0, color='gray', linestyle=':', linewidth=1)

# 任意斜率参考线（贯穿整张图）
ax.axline((0, 0), (1, 1), color='red', linestyle='--', linewidth=1.5)

ax.legend()
fig.savefig('reference_lines.png')
```

### 箭头标注

```python
from rsplotlib import pyplot as plt

fig, ax = plt.subplots()

# 绘制曲线
x = list(range(-10, 11))
y = [xi**3 - 3*xi for xi in x]
ax.plot(x, y, 'b-', lw=2)

# 添加箭头标注
ax.annotate('局部极大值', xy=(-1, 2), xytext=(-8, 500),
            fontsize=11, color='red',
            arrowprops={'arrowstyle': '->', 'arrowsize': 1.0})

ax.annotate('局部极小值', xy=(1, -2), xytext=(3, -500),
            fontsize=11, color='blue',
            arrowprops={'arrowstyle': '->', 'arrowsize': 1.0})

ax.annotate('原点', xy=(0, 0), xytext=(5, 200),
            fontsize=11, color='darkgreen')

ax.set_title('ax.annotate：带箭头的文本标注')
fig.savefig('annotations.png')
```

### 批量水平线/垂直线（Rust 层实现）

```python
from rsplotlib import pyplot as plt

fig, ax = plt.subplots()
ax.plot([0, 10], [0, 10], 'k-', lw=1)

# 批量绘制水平线（Rust 层循环，Python 零开销）
ax.hlines([2, 4, 6, 8], color='steelblue', linestyle='--', linewidth=1)

# 批量绘制垂直线
ax.vlines([2, 4, 6, 8], color='darkorange', linestyle=':', linewidth=1)

ax.set_title('hlines / vlines：批量参考线（Rust 层实现）')
fig.savefig('hlines_vlines.png')
```

### 多子图

```python
from rsplotlib import pyplot as plt

# 2x2 网格
fig, axes = plt.subplots(2, 2)

# axes 是一个扁平列表
axes[0].plot([1, 2, 3], [1, 4, 9])
axes[0].set_title('子图 1')

axes[1].bar(['A', 'B', 'C'], [3, 7, 2])
axes[1].set_title('子图 2')

axes[2].scatter([1,2,3,4], [4,3,2,1], c=['red','green','blue','orange'])
axes[2].set_title('子图 3')

axes[3].hist([0.5, 1.2, 1.8, 2.1, 2.5, 3.0, 3.2, 3.8, 4.1, 4.5], bins=5)
axes[3].set_title('子图 4')

fig.savefig('subplots.png', dpi=200)
```

### 模块级接口（无需显式创建 Axes）

```python
from rsplotlib import pyplot as plt

# 直接使用 plt.* 接口
plt.figure()
plt.plot([1, 2, 3], [1, 2, 3], 'r-')
plt.axhline(y=2, color='gray', linestyle='--')
plt.axvspan(1, 2, color='yellow', alpha=0.3)
plt.title('模块级接口')
plt.savefig('module_level.png')
```

---

## 功能清单

### 绘图函数

| 函数             | 说明                        | 模块级接口 | Axes 接口 |
| ---------------- | --------------------------- | ---------- | --------- |
| `plot()`         | 折线图                      | ✅         | ✅        |
| `scatter()`      | 散点图（支持颜色/大小数组） | ✅         | ✅        |
| `bar()`          | 柱状图                      | ✅         | ✅        |
| `barh()`         | 水平柱状图                  | ✅         | ✅        |
| `hist()`         | 直方图                      | ✅         | ✅        |
| `pie()`          | 饼图                        | ✅         | ✅        |
| `boxplot()`      | 箱线图                      | ✅         | ✅        |
| `fill_between()` | 曲线间填充                  | ✅         | ✅        |
| `errorbar()`     | 误差棒图                    | ✅         | ✅        |
| `stem()`         | 茎叶图                      | ✅         | ✅        |
| `step()`         | 阶梯图                      | ✅         | ✅        |
| `imshow()`       | 图像/热力图                 | ✅         | ✅        |
| `stackplot()`    | 堆叠面积图                  | ✅         | ✅        |
| `semilogx()`     | x 轴对数坐标折线图          | ✅         | ✅        |
| `semilogy()`     | y 轴对数坐标折线图          | ✅         | ✅        |
| `loglog()`       | 双对数坐标折线图            | ✅         | ✅        |

### 辅助元素

| 函数         | 说明                 | 模块级接口 | Axes 接口 |
| ------------ | -------------------- | ---------- | --------- |
| `axhline()`  | 水平参考线           | ✅         | ✅        |
| `axvline()`  | 垂直参考线           | ✅         | ✅        |
| `axhspan()`  | 水平区间填充         | ✅         | ✅        |
| `axvspan()`  | 垂直区间填充         | ✅         | ✅        |
| `axline()`   | 任意斜率参考线       | ✅         | ✅        |
| `hlines()`   | 批量水平线 (Rust 层) | ✅         | ✅        |
| `vlines()`   | 批量垂直线 (Rust 层) | ✅         | ✅        |
| `text()`     | 文本标注             | ✅         | ✅        |
| `annotate()` | 带箭头文本标注       | ✅         | ✅        |

### 图表配置

| 函数                                   | 说明                                           |
| -------------------------------------- | ---------------------------------------------- |
| `title()` / `ax.set_title()`           | 设置图表标题（`loc='left'/'center'/'right'`）  |
| `xlabel()` / `ax.set_xlabel()`         | 设置 X 轴标签（`loc='left'/'center'/'right'`） |
| `ylabel()` / `ax.set_ylabel()`         | 设置 Y 轴标签（`loc='top'/'center'/'bottom'`） |
| `grid()`                               | 显示/隐藏网格                                  |
| `legend()`                             | 显示图例                                       |
| `xlim()` / `ylim()`                    | 设置坐标轴范围                                 |
| `xticks()` / `yticks()`                | 设置刻度位置和标签                             |
| `xscale()` / `yscale()`                | 设置坐标缩放（`linear` / `log`）               |
| `margins()`                            | 设置自动缩放边距                               |
| `box()`                                | 设置坐标轴边框显示                             |
| `minorticks_on()` / `minorticks_off()` | 次要刻度显示控制                               |

### 子图与布局

| 函数                           | 说明             |
| ------------------------------ | ---------------- |
| `subplots(nrows, ncols)`       | 创建子图网格     |
| `subplot(nrows, ncols, index)` | 创建单个子图     |
| `twinx()` / `twiny()`          | 创建双坐标轴     |
| `tight_layout()`               | 自动调整布局     |
| `fig.set_size(w, h)`           | 设置图形像素尺寸 |

### 图形控制

| 函数                          | 说明                       |
| ----------------------------- | -------------------------- |
| `figure()`                    | 创建新 Figure 对象         |
| `savefig(filename, dpi=None)` | 保存到文件，支持自定义 DPI |
| `show()`                      | 显示图形（保存到默认位置） |
| `gca()`                       | 获取当前 Axes              |
| `gcf()`                       | 获取当前 Figure            |
| `cla()`                       | 清空当前 Axes              |
| `clf()`                       | 清空当前 Figure            |
| `close()`                     | 关闭当前 Figure            |

---

## API 参考

### Figure.savefig

保存图形到文件。

```python
fig.savefig(filename, dpi=None)
```

**参数：**

- `filename` (str): 输出文件路径。支持 `.png` 和 `.svg` 扩展名
- `dpi` (float, 可选): 分辨率（每英寸点数）。默认使用 Figure 创建时的 DPI

**示例：**

```python
fig.savefig('plot.png')              # 默认 DPI
fig.savefig('plot_hd.png', dpi=150)  # 屏幕分辨率
fig.savefig('plot_print.png', dpi=300)  # 印刷分辨率
fig.savefig('plot.svg')              # 矢量图
```

### Axes.scatter — 增强版

散点图，支持每点独立颜色/大小。

```python
ax.scatter(x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0, **kwargs)
```

**参数：**

- `x`, `y` (list/array): 点坐标数据
- `s` (float 或 list/array): 单个浮点值用于所有点，或数组表示每点大小
- `c` (str 或 list[str]): 单个颜色字符串，或颜色字符串数组
- `marker` (str): 标记形状，支持 `'o'`, `'s'`, `'^'`, `'v'`, `'D'`, `'*'`, `'+'`, `'x'`, `'<'`, `'>'`
- `label` (str): 图例标签
- `alpha` (float): 透明度 (0.0-1.0)
- `**kwargs`: 额外参数，支持 `color`（作为 `c` 的别名）

**示例：**

```python
# 单色散点
ax.scatter(x, y, c='red', s=50)

# 每点独立颜色
ax.scatter(x, y, c=['red', 'green', 'blue', ...], s=50)

# 每点独立大小
ax.scatter(x, y, s=[10, 20, 30, ...], c='blue')

# 颜色+大小同时独立（Rust 层批量处理）
ax.scatter(x, y, c=colors, s=sizes, marker='D', alpha=0.8)
```

### Axes.axhspan / axvspan

区间高亮填充。

```python
ax.axhspan(ymin, ymax, color=None, alpha=0.3)
ax.axvspan(xmin, xmax, color=None, alpha=0.3)
```

**参数：**

- `ymin`, `ymax` (float): 水平区间的 y 轴上下界（数据坐标）
- `xmin`, `xmax` (float): 垂直区间的 x 轴左右界（数据坐标）
- `color` (str): 填充颜色，默认浅蓝灰色
- `alpha` (float): 透明度，默认 0.3

### Axes.axline

通过两点绘制贯穿全图的任意斜率参考线。

```python
ax.axline(xy1, xy2, color=None, linestyle=None, linewidth=None)
```

**参数：**

- `xy1` (tuple): 起点坐标 `(x1, y1)`
- `xy2` (tuple): 终点坐标 `(x2, y2)`
- `color` (str): 线颜色
- `linestyle` (str): 线型，`'-'` (实线), `'--'` (虚线), `':'` (点线), `'-.'` (点划线)
- `linewidth` (float): 线宽

### Axes.annotate

添加带箭头的文本标注。

```python
ax.annotate(text, xy, xytext=None, fontsize=12.0, color='black',
            arrowprops=None, arrowstyle=None, arrowsize=1.0)
```

**参数：**

- `text` (str): 标注文本
- `xy` (tuple): 被标注点坐标 `(x, y)`
- `xytext` (tuple, 可选): 文本放置位置。若提供，自动从该位置绘制箭头到 `xy`
- `fontsize` (float): 字体大小，默认 12.0
- `color` (str): 文本和箭头颜色，默认 `'black'`
- `arrowprops` (dict, 可选): 箭头属性字典，支持：
  - `arrowstyle`: 箭头样式（如 `'->'`, `'-|>'`）
  - `arrowsize`: 箭头相对大小
- `arrowstyle` (str, 可选): 独立于 `arrowprops` 的箭头样式
- `arrowsize` (float, 可选): 独立于 `arrowprops` 的箭头大小

### hlines / vlines（Rust 层批量实现）

绘制多条水平线或垂直线。所有内部循环在 Rust 层完成，避免 Python 级循环开销。

```python
ax.hlines(y, color=None, linestyle=None, linewidth=None)
ax.vlines(x, color=None, linestyle=None, linewidth=None)
```

**参数：**

- `y` / `x` (list/array): 位置列表

---

## 性能优势

rsplotlib 在架构设计上通过"分层下沉"策略实现性能优化：

### 架构分层

```
┌──────────────────────────────────────────────┐
│  Python 层 (API 兼容层)                      │
│  · pyplot.py   (Matplotlib 兼容接口)         │
│  · api.py      (参数规范化 / 别名映射)       │
│  · _patch_*    (方法补丁 / 动态分发)         │
├──────────────────────────────────────────────┤
│  Rust 层 (高性能核心)                        │
│  · lib.rs      (模块注册 / 字体系统)         │
│  · figure.rs   (Figure 对象 / 渲染调度)      │
│  · axes.rs     (Axes 对象 / 数据解析)        │
│  · elements.rs (绘图元素数据结构)            │
│  · axes_render_elements.rs (plotters 渲染)   │
│  · pyfuncs.rs  (模块级函数暴露)              │
└──────────────────────────────────────────────┘
```

### 性能关键路径下沉到 Rust

| 功能                        | 传统实现                            | rsplotlib 实现                  |
| --------------------------- | ----------------------------------- | ------------------------------- |
| `scatter(c=colors)`         | Python 层遍历每个点                 | Rust 层 `ScatterMulti` 统一渲染 |
| `scatter(s=sizes)`          | Python 层遍历每个点                 | Rust 层统一大小数组处理         |
| `hlines([y1, y2, y3, ...])` | Python `for` 循环多次调用 `axhline` | Rust 层单次调用批量处理         |
| `vlines([x1, x2, x3, ...])` | Python `for` 循环多次调用 `axvline` | Rust 层单次调用批量处理         |
| `savefig(dpi=300)`          | 无 DPI 支持 / Python 层缩放         | Rust 层直接写入 PNG DPI 元数据  |

### 为什么选择 Rust + Python 混合架构

1. **开发效率**：Python 层快速迭代 API 设计、参数校验
2. **执行性能**：Rust 层处理渲染、循环、内存密集型计算
3. **内存安全**：Rust 编译期保证无数据竞争
4. **零额外依赖**：无原生 Matplotlib 安装要求

---

## 项目结构

```
rsplotlib/
├── python/                          # Python 包装层
│   └── rsplotlib/
│       ├── __init__.py              # 包入口与导出
│       ├── api.py                   # 模块级 API 函数定义
│       ├── pyplot.py                # Matplotlib 兼容 pyplot 接口
│       │                            # (含中文 docstring, IDE 悬停提示)
│       ├── pylab.py                 # pylab 风格接口，提供 mpl.rcParams
│       ├── style.py                 # plt.style 样式管理
│       ├── _rcparams.py             # rcParams 配置管理
│       ├── _font_resolver.py        # 系统字体路径解析
│       ├── _figure_defaults.py      # Figure 默认配置
│       ├── gridspec.py              # 网格布局管理
│       └── ticker.py                # 刻度定位器 (AutoLocator, MaxNLocator)
│
├── src/                             # Rust 核心实现
│   ├── lib.rs                       # 库入口，Python 模块注册
│   ├── figure.rs                    # Figure 类 (savefig, DPI 管理)
│   ├── axes.rs                      # Axes 类 (所有绘图方法)
│   ├── axes_render_elements.rs      # 元素渲染引擎 (基于 plotters)
│   ├── axes_bounds.rs               # 坐标边界计算
│   ├── axes_title.rs                # 标题渲染
│   ├── axes_legend.rs               # 图例渲染
│   ├── axes_grid.rs                 # 网格线渲染
│   ├── axes_mesh.rs                 # 坐标轴刻度
│   ├── axis.rs                      # Axis 数据结构
│   ├── elements.rs                  # 绘图元素枚举 (Line/Scatter/Bar/...)
│   ├── pyfuncs.rs                   # 暴露给 Python 的模块级函数
│   ├── colors.rs                    # 颜色解析 (命名色, RGB, HSL)
│   ├── colormap.rs                  # 颜色映射
│   ├── marker.rs                    # 标记形状渲染
│   └── text_utils.rs                # 文本工具函数
│
├── Cargo.toml                       # Rust 依赖 (PyO3, plotters, png)
├── pyproject.toml                   # Python 包配置 (maturin)
├── build_wheel.sh                   # wheel 包构建脚本
├── README.md                        # 英文说明
└── README_zh.md                     # 中文说明（本文档）
```

---

## 开发与贡献

### 本地开发环境

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装 maturin
pip install maturin

# 3. 开发模式构建（自动编译 Rust 并安装到当前 Python 环境）
maturin develop --release

# 4. 运行交互式测试
python3 -c "
from rsplotlib import pyplot as plt
fig, ax = plt.subplots()
ax.plot([1,2,3], [1,4,9])
fig.savefig('/tmp/test.png')
print('OK')
"
```

### 开发工作流

1. **理解需求**：分析需要实现的 Matplotlib 功能
2. **Rust 层实现**：在 `src/` 中添加核心逻辑
   - 新增绘图元素：修改 `elements.rs`
   - 添加绘图方法：修改 `axes.rs`
   - 添加渲染逻辑：修改 `axes_render_elements.rs`
   - 暴露模块级函数：修改 `pyfuncs.rs`
   - 注册：修改 `lib.rs`
3. **Python 层包装**：在 `python/rsplotlib/` 中添加 API
   - 添加中文 docstring（供 IDE 悬停显示）
   - 处理参数别名（如 `lw` → `linewidth`）
   - 将数组参数路由到 Rust 层批量方法
4. **编译测试**：`maturin develop --release`
5. **验证结果**：生成图像进行目视检查

### 为 Python 层添加文档（IDE 悬停）

Python 包装层的函数都包含详细的中文 docstring，当用户在 IDE（VS Code、PyCharm 等）中悬停在函数上时，会自动显示中文说明和参数列表。

```python
def axhspan(ymin, ymax, **kwargs):
    """绘制水平方向的区间填充。

    用法:
        plt.axhspan(0, 1, color='yellow', alpha=0.3)

    Args:
        ymin: y 轴下限
        ymax: y 轴上限
        color: 填充颜色 (默认蓝灰色)
        alpha: 透明度 (0.0-1.0, 默认 0.3)
    """
    ...
```

### 贡献指南

欢迎提交 PR！请在提交时包含：

- 清晰的功能说明或问题修复描述
- 适用时包含生成的示例图像
- 确保同时更新 `README.md` 和 `README_zh.md`

### 已知限制

- 当前版本不支持 3D 绘图
- 不支持动画/交互式图表
- `contour` / `violinplot` / `hexbin` 为占位实现

---

## 字体配置

rsplotlib 支持通过 `mpl.rcParams` 自定义字体，同时支持直接注册字体文件。

### 自动检测的系统字体

| 平台        | 常用字体                                                                   |
| ----------- | -------------------------------------------------------------------------- |
| **macOS**   | Arial, Helvetica, PingFang SC, STHeiti, Hiragino Sans GB, Arial Unicode MS |
| **Linux**   | DejaVu Sans, Liberation Sans, Noto Sans CJK SC, WenQuanYi Micro Hei        |
| **Windows** | Microsoft YaHei, SimHei, SimSun, Arial                                     |

### 自定义字体配置

```python
from rsplotlib.pylab import mpl
from rsplotlib import pyplot as plt

# 设置 sans-serif 字体族（优先顺序）
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Microsoft YaHei', 'DejaVu Sans']

# 设置默认字体大小
mpl.rcParams['font.size'] = 12
```

### 直接注册字体文件

```python
from rsplotlib import rsplotlib as _rs

# 从任意 .ttf/.otf/.ttc 文件注册为 sans-serif 字体
_rs.register_sans_serif_font('/path/to/your/custom-font.ttf')
```

---

## 许可证

MIT License — 详见 [LICENSE](LICENSE) 文件。

---

## 致谢

- [**PyO3**](https://github.com/PyO3/pyo3) — Rust ↔ Python 绑定库，版本 0.29
- [**plotters**](https://github.com/plotters-rs/plotters) — Rust 绘图库，提供底层渲染能力
- [**Matplotlib**](https://matplotlib.org/) — Python 绘图生态标准，提供 API 设计参考
- [**maturin**](https://github.com/PyO3/maturin) — Rust Python 包构建与发布工具

---

## 相关链接

- **GitHub**: https://github.com/YJ-Niu/rsplotlib

---

_最后更新：2026-07-03 · 版本 v0.1.9_
