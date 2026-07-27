# rsplotlib 项目开发规则

## 项目概述

rsplotlib 是一个使用 **Rust + PyO3** 开发的高性能 Python 绘图库，提供与 **matplotlib** 兼容的 API 接口。

- **Rust 层** (`src/`): 负责底层核心实现，包括图形渲染、颜色处理、字体解析、布局计算等
- **Python 层** (`python/rsplotlib/`): 负责公开 API 接口，方法参数默认值要全面，代理调用 Rust 底层实现
- **构建工具**: 使用 `maturin` 构建 wheel，通过 `build_wheel.sh` 脚本构建并安装到 `.venv`

## 开发环境

- Python 虚拟环境: `.venv` (使用 uv 创建)
- 构建命令: `./build_wheel.sh --debug` (调试模式) 或 `./build_wheel.sh` (release 模式)
- Rust 工具链: 由 `rust-toolchain.toml` 固定，当前为 edition 2024

## 关键约定

### 1. Debug 目录

- 所有调试、分析、验证、测试代码及生成的结果必须放在 `/Users/user/Desktop/rust_project/rsplotlib/debug/` 目录中
- 禁止在项目根目录或其他位置创建零散的测试文件

### 2. 代码修改范围

- **可以修改**: `rsplotlib` 项目内的代码 (`src/`, `python/rsplotlib/`, `pyproject.toml`, `Cargo.toml`、"test/test_rf/networkx"、"test/test_rf/scipy"、"test/test_rf/skrf"等)
- **禁止修改**: 用户测试代码、`.venv` 环境中的任何代码
- 修改后使用 `build_wheel.sh` 构建并安装到 `.venv` 进行测试

### 3. 修复优先级

- 优先在 **Rust 层** (`src/`) 实现底层修复
- 然后在 **Python 层** (`python/rsplotlib/`) 补全和优化方法接口
- 确保 Python 接口的默认参数全面，与 matplotlib 行为一致

### 4. 依赖库处理

- **禁止**安装 `numpy` 或其他第三方 Python 库
- 使用 `rsnumpy` (Rust + PyO3 开发的 numpy 兼容库) 替代 numpy
- 如果 `rsnumpy` 功能不全，提出如何更新优化 `rsnumpy` 的建议，而不是安装 numpy

### 5. 代码风格

- Rust 代码: 遵循 `rustfmt` 和 `clippy` 规范（构建脚本会自动检查）
- Python 代码: 遵循 PEP 8 规范
- 提交前确保 `cargo fmt --all -- --check` 和 `cargo clippy --all-targets -- -D warnings` 通过

## 6. 修复后的构建与测试

- 每次修改后，立即执行 `./build_wheel.sh` 重新构建并安装。
- 运行项目提供的测试用例（如 `pytest`）确保修复有效。若测试失败，继续修改直到通过。

## 7. 长期优化原则（非强制）

- 当发现性能瓶颈来自 Python 层循环时，应评估将其迁移到 Rust 实现的可行性，但**本次修复不必强制执行**。

## 项目结构

```
rsplotlib/
├── src/                    # Rust 源码
│   ├── lib.rs              # 库入口，PyO3 模块注册
│   ├── core/               # 核心模块（颜色、元素、标记、颜色映射）
│   ├── figure/             # 图形模块（Figure、Axes、Axis、图例、标题等）
│   ├── layout/             # 布局模块（GridSpec）
│   ├── ticks/              # 刻度模块（Ticker）
│   └── utils/              # 工具模块（字体、样式、数学文本、RGB 后端等）
├── python/                 # Python 源码
│   └── rsplotlib/          # Python 包
│       ├── __init__.py     # 包入口，导出核心 API
│       ├── pyplot.py       # matplotlib.pyplot 兼容接口
│       ├── core/api.py     # 核心 API 定义
│       ├── figure/         # 图形相关 Python 接口
│       ├── colors.py       # 颜色接口
│       ├── text.py         # 文本接口
│       ├── patches.py      # 图形块接口
│       ├── collections.py  # 集合接口
│       ├── cm.py           # 颜色映射接口
│       └── ...
├── build_wheel.sh          # 构建脚本
├── pyproject.toml          # 项目配置
└── Cargo.toml              # Rust 配置
```
