# rsplotlib

一个使用 Rust 实现的高性能 Python 绘图库，旨在兼容 Matplotlib 的 API，同时利用 Rust 的性能优势和可移植性。

## 特性

- **Matplotlib 兼容 API**: 支持常见的绘图函数如 `plot`、`scatter`、`bar`、`hist`、`pie` 等
- **高性能**: 核心渲染引擎使用 Rust 编写，基于 plotters 库
- **跨平台**: 支持 macOS、Linux 和 Windows
- **自定义字体支持**: 通过 `mpl.rcParams` 支持字体自定义
- **多后端支持**: 支持 SVG 和 PNG 输出

## 安装

### 前置依赖

- Python 3.9+
- Rust 1.70+（从源码编译时需要）
- uv（推荐的 Python 包管理工具）

### 从源码构建

```bash
# 创建虚拟环境
uv venv --python 3.13

# 激活虚拟环境
source .venv/bin/activate

# 构建 Rust 扩展并安装包
./build_wheel.sh

# 安装依赖
uv pip install matplotlib numpy
```

## 快速开始

```python
from rsplotlib import pyplot as plt
from rsplotlib.pylab import mpl

# 配置字体（可选）
mpl.rcParams['font.sans-serif'] = ['Arial']

# 创建图形
fig, ax = plt.subplots()

# 绘制数据
ax.plot([0, 1, 2, 3], [1, 4, 2, 3], label='折线')
ax.scatter([0, 1, 2, 3], [2, 3, 1, 4], color='red', label='散点')

# 添加标签和标题
ax.set_xlabel('X轴')
ax.set_ylabel('Y轴')
ax.set_title('示例图')
ax.legend()

# 保存图形
fig.savefig('output.png')
```

## 支持的函数

### 绘图函数
- `plot()` - 折线图
- `scatter()` - 散点图
- `bar()` - 柱状图
- `barh()` - 水平柱状图
- `hist()` - 直方图
- `pie()` - 饼图
- `boxplot()` - 箱线图
- `fill_between()` - 填充区域
- `errorbar()` - 误差棒图
- `stem()` - 茎叶图
- `step()` - 阶梯图
- `imshow()` - 图像显示

### 文本函数
- `text()` - 添加文本
- `title()` - 添加标题
- `xlabel()` - 添加 X 轴标签
- `ylabel()` - 添加 Y 轴标签

### 配置函数
- `grid()` - 切换网格显示
- `legend()` - 显示图例
- `xlim()` / `ylim()` - 设置坐标轴范围
- `xticks()` / `yticks()` - 设置刻度位置
- `xscale()` / `yscale()` - 设置坐标轴缩放（线性/对数）
- `use()` - 设置后端

### 子图函数
- `subplots()` - 创建子图网格
- `subplot()` - 创建单个子图
- `twinx()` / `twiny()` - 创建双坐标轴
- `tight_layout()` - 自动调整布局

## 字体配置

rsplotlib 支持通过 `mpl.rcParams` 自定义字体：

```python
from rsplotlib.pylab import mpl

# 设置字体族
mpl.rcParams['font.sans-serif'] = ['PingFang SC', 'Microsoft YaHei', 'DejaVu Sans']

# 设置字体大小
mpl.rcParams['font.size'] = 12
```

### 支持的字体
- **macOS**: Arial, Helvetica, PingFang SC, STHeiti, Hiragino Sans GB
- **Linux**: DejaVu Sans, Liberation Sans, Noto Sans CJK SC, WenQuanYi Micro Hei
- **Windows**: Microsoft YaHei, SimHei, SimSun

## 项目结构

```
rsplotlib/
├── python/
│   └── rsplotlib/          # Python 封装实现
│       ├── __init__.py      # 包导出
│       ├── api.py           # 公共 API 定义
│       ├── pyplot.py        # pyplot 模块（兼容 Matplotlib）
│       ├── pylab.py         # pylab 模块，包含 mpl.rcParams
│       ├── _rcparams.py     # rcParams 配置管理
│       └── _font_resolver.py # 字体路径解析
├── src/                     # Rust 实现
│   ├── lib.rs              # Rust 库入口
│   ├── pyfuncs.rs          # 暴露给 Python 的函数
│   ├── figure.rs           # Figure 实现
│   ├── axes.rs             # Axes 实现
│   └── axes_render_elements.rs # 渲染元素
├── Cargo.toml              # Rust 依赖配置
├── pyproject.toml          # Python 包配置
└── build_wheel.sh          # 构建脚本
```

## 开发流程

1. **修复（Fix）**: 根据测试用例定位并修复问题
2. **重构（Refactor）**: 清理代码，优先修改 `python/rsplotlib/` 下的代码
3. **画图（Plot）**: 使用 `main.py` 或 `python/examples.py` 生成对比图像
4. **对比（Compare）**: 将生成的 PNG 与 Matplotlib 参考图像对比
5. **迭代（Iterate）**: 根据差异重复循环

## 贡献指南

提交 PR 时请包含：
- 生成的示例图像
- 修复的问题或改进的说明

## 许可证

MIT License

## 致谢

- [plotters](https://github.com/plotters-rs/plotters) - Rust 绘图库
- [Matplotlib](https://matplotlib.org/) - Python 绘图库，API 参考来源