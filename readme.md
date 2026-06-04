项目简介
本项目（rsplotlib）是一个 Rust 实现的 Python 绘图库，目标兼容部分 Matplotlib 的 API，并使用 Rust 的绘图库（如 plotters）提高性能与可移植性。

快速开始

- 建议在虚拟环境中操作 uv创建python环境，如 `uv venv --python 3.13`。
- 构建并安装库：

```bash
./build_wheel.sh    # 构建 wheel 到 ./wheels 或 target 下
```

- 依赖：项目主要依赖 `matplotlib`（用于对比与测试），请在虚拟环境中安装：

```bash
uv pip install -r requirements.txt  # 如果存在 requirements.txt
uv pip install matplotlib
```

开发与调试流程（推荐循环）

1. 修复（Fix）: 根据测试用例或示例运行输出定位问题并修复。
2. 重构（Refactor）: 将修复后的代码整理为更清晰的模块（优先修改 `python/rsplotlib` 下的代码）。
3. 画图（Plot）: 使用 `main.py` 或 `python/examples.py` 生成对比图像，输出目录为 `plots/` 或 `N238B W1-plots/`。
4. 对比（Compare）: 将生成的 SVG/PNG 与使用 Matplotlib 原生生成的图片逐一比较，记录差异（布局、字体、间距、图例、刻度等）。
5. 再修复（Iterate）: 基于差异继续修复并自动进入下一轮。

主要关注点

- 目标并非逐字复刻 Matplotlib 的实现，而是在兼容其常用 API 的前提下尽量匹配输出效果。
- 注意 SVG 与 PNG 的渲染差异；对于 保存SVG不要过多的修改，避免做会影响 raster 输出（PNG）布局的改动。
- 保持代码简洁：优先实现常用、稳定的接口，避免过度复杂化。

重要文件

- `main.py`：项目示例/测试入口，负责批量生成测试图像（查看并运行以生成 `plots/` 下的图像）。
- `python/rsplotlib/`：Python 封装实现，主要工作区。
- `testing/`：测试脚本集合。
- `python/rsplotlib/`: python库接口
- `src/'`：Rust 实现的绘图库，主要工作区。

如何执行第一轮（快速验证）

```bash
# 1. 激活虚拟环境
source .venv/bin/activate

# 2. 安装依赖并安装本包（如未安装）
uv pip install -r requirements.txt || true
./build_wheel.sh

# 3. 运行示例脚本以生成图像
uv run python main.py

# 生成的图像位于 plots/ 或 N238B W1-plots/，与 Matplotlib 输出进行对比。
```

循环迭代目标

- 每次修改后，提交最小可复现的变更并运行 `main.py` 生成对比图像。
- 使用差异化检查（手动或脚本化）记录明显的不一致项，优先修复影响视觉结果与坐标系的错误。

贡献

- 提交 PR 时请附带生成的示例图像以及说明本次改动修复/改进了哪些问题。

顺序循环原则：修复一部分问题后 → 重构 → 画图 → 对比 → 再修复（自动重复迭代）
