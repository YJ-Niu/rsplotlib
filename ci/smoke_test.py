"""CI 冒烟测试：不依赖 rsnumpy，可在任意平台/Python 版本上运行。

覆盖 pyplot 公共 API 的主要路径，并重点验证 title/xlabel/ylabel 的 loc 定位
以及 png/svg 两种后端。任一步骤失败即以非零状态退出，供 CI 判定。
"""
import os
import tempfile

import rsplotlib.pyplot as plt


def _check(path):
    assert os.path.exists(path), f"输出文件未生成: {path}"
    assert os.path.getsize(path) > 0, f"输出文件为空: {path}"


def main():
    out = tempfile.mkdtemp(prefix="rsplotlib_ci_")

    # 折线图 + 标题/坐标轴标签 loc 定位（本次修复的功能）
    plt.plot([0, 1, 2, 3], [3, 7, 5, 9])
    plt.plot([0, 1, 2, 3], [6, 2, 13, 10])
    plt.title("CI smoke", loc="left")
    plt.xlabel("x", loc="right")
    plt.ylabel("y", loc="top")
    p = os.path.join(out, "line.png")
    plt.savefig(p)
    plt.close("all")
    _check(p)

    # title / xlabel 的每个 loc 取值都要能跑通
    for loc in ("left", "center", "right"):
        plt.plot([1, 2, 3], [1, 4, 9])
        plt.title(f"title-{loc}", loc=loc)
        plt.xlabel("x", loc=loc)
        p = os.path.join(out, f"title_{loc}.png")
        plt.savefig(p)
        plt.close("all")
        _check(p)

    # ylabel 的每个 loc 取值
    for loc in ("bottom", "center", "top"):
        plt.plot([1, 2, 3], [1, 4, 9])
        plt.ylabel("y", loc=loc)
        p = os.path.join(out, f"ylabel_{loc}.png")
        plt.savefig(p)
        plt.close("all")
        _check(p)

    # 散点 / 柱状
    plt.scatter([1, 2, 3], [3, 1, 2])
    p = os.path.join(out, "scatter.png")
    plt.savefig(p)
    plt.close("all")
    _check(p)

    plt.bar([0, 1, 2], [1, 3, 2])
    p = os.path.join(out, "bar.png")
    plt.savefig(p)
    plt.close("all")
    _check(p)

    # SVG 矢量后端
    plt.plot([0, 1, 2], [0, 1, 4])
    p = os.path.join(out, "line.svg")
    plt.savefig(p)
    plt.close("all")
    _check(p)

    print("CI smoke test passed:", out)


if __name__ == "__main__":
    main()
