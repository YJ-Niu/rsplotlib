"""rsplot 使用示例

这个模块提供 rsplot 库的详细使用示例。

使用方式：
>>> from rsplot import examples
>>> examples.run_example('line')  # 运行指定示例
>>> examples.run_all()  # 运行所有示例（会生成图片文件）
"""

import rsplot as plt
import rsnum as np
from rsnum import random


def example_line_plot():
    """折线图示例"""
    print("=" * 60)
    print("折线图示例")
    print("=" * 60)
    
    x = np.linspace(0, 10, 100)
    
    plt.figure()
    plt.plot(x, np.sin(x), label='sin(x)', color='blue')
    plt.plot(x, np.cos(x), label='cos(x)', color='red', linestyle='--')
    plt.xlabel('x')
    plt.ylabel('y')
    plt.title('正弦和余弦曲线')
    plt.legend('upper right')
    plt.grid(True)
    plt.savefig('/tmp/rsplot_example_line.svg')
    print("已保存: /tmp/rsplot_example_line.svg")


def example_scatter():
    """散点图示例"""
    print("\n" + "=" * 60)
    print("散点图示例")
    print("=" * 60)
    
    random.seed(42)
    x = random.randn(100)
    y = random.randn(100)
    
    plt.figure()
    plt.scatter(x, y, color='green', s=10)
    plt.xlabel('X')
    plt.ylabel('Y')
    plt.title('散点图示例')
    plt.grid(True)
    plt.savefig('/tmp/rsplot_example_scatter.svg')
    print("已保存: /tmp/rsplot_example_scatter.svg")


def example_bar():
    """柱状图示例"""
    print("\n" + "=" * 60)
    print("柱状图示例")
    print("=" * 60)
    
    categories = ['A', 'B', 'C', 'D', 'E']
    values = np.array([3, 7, 2, 5, 8])
    
    plt.figure()
    for i in range(len(values)):
        plt.bar(np.array([i]), np.array([values[i]]), color='#1f77b4')
    plt.xticks([0, 1, 2, 3, 4], categories)
    plt.title('柱状图')
    plt.savefig('/tmp/rsplot_example_bar.svg')
    print("已保存: /tmp/rsplot_example_bar.svg")


def example_histogram():
    """直方图示例"""
    print("\n" + "=" * 60)
    print("直方图示例")
    print("=" * 60)
    
    random.seed(42)
    data = random.randn(1000)
    
    plt.figure()
    plt.hist(data, bins=30, color='steelblue')
    plt.xlabel('值')
    plt.ylabel('频数')
    plt.title('直方图 (1000个样本)')
    plt.savefig('/tmp/rsplot_example_hist.svg')
    print("已保存: /tmp/rsplot_example_hist.svg")


def example_subplots():
    """子图示例"""
    print("\n" + "=" * 60)
    print("子图示例")
    print("=" * 60)
    
    x = np.linspace(0, 10, 100)
    
    fig, axes = plt.subplots(2, 2)
    
    axes[0].plot(x, np.sin(x), color='blue')
    axes[0].set_title('sin(x)')
    
    axes[1].plot(x, np.cos(x), color='red')
    axes[1].set_title('cos(x)')
    
    axes[2].plot(x, np.exp(-x/2), color='green')
    axes[2].set_title('exp(-x/2)')
    
    axes[3].plot(x, np.tan(x), color='purple')
    axes[3].set_title('tan(x)')
    axes[3].set_ylim(-5, 5)
    
    plt.savefig('/tmp/rsplot_example_subplots.svg')
    print("已保存: /tmp/rsplot_example_subplots.svg")


def example_pie():
    """饼图示例"""
    print("\n" + "=" * 60)
    print("饼图示例")
    print("=" * 60)
    
    sizes = np.array([15, 30, 45, 10])
    labels = ['A', 'B', 'C', 'D']
    
    plt.figure()
    plt.pie(sizes, labels=labels, autopct=True)
    plt.title('饼图')
    plt.savefig('/tmp/rsplot_example_pie.svg')
    print("已保存: /tmp/rsplot_example_pie.svg")


def example_boxplot():
    """箱线图示例"""
    print("\n" + "=" * 60)
    print("箱线图示例")
    print("=" * 60)
    
    random.seed(42)
    data1 = random.randn(100)
    data2 = random.randn(100) * 1.5 + 1
    data3 = random.randn(100) * 0.8 - 1
    
    plt.figure()
    plt.boxplot([data1.tolist(), data2.tolist(), data3.tolist()],
                labels=['Group A', 'Group B', 'Group C'])
    plt.title('箱线图')
    plt.savefig('/tmp/rsplot_example_boxplot.svg')
    print("已保存: /tmp/rsplot_example_boxplot.svg")


def example_errorbar():
    """误差棒图示例"""
    print("\n" + "=" * 60)
    print("误差棒图示例")
    print("=" * 60)
    
    x = np.arange(0, 10, 2)
    y = np.array([1, 4, 3, 6, 5])
    yerr = np.array([0.5, 0.8, 0.6, 1.0, 0.7])
    
    plt.figure()
    plt.errorbar(x, y, yerr=yerr, color='blue', capsize=5)
    plt.xlabel('x')
    plt.ylabel('y')
    plt.title('误差棒图')
    plt.grid(True)
    plt.savefig('/tmp/rsplot_example_errorbar.svg')
    print("已保存: /tmp/rsplot_example_errorbar.svg")


def example_twin_axis():
    """双轴示例"""
    print("\n" + "=" * 60)
    print("双轴示例")
    print("=" * 60)
    
    x = np.linspace(0, 10, 100)
    y1 = np.sin(x)
    y2 = np.exp(x/5)
    
    plt.figure()
    plt.plot(x, y1, color='blue', label='sin(x)')
    plt.xlabel('x')
    plt.ylabel('sin(x)', color='blue')
    
    ax2 = plt.twinx()
    ax2.plot(x, y2, color='red', label='exp(x/5)')
    ax2.set_ylabel('exp(x/5)', color='red')
    
    plt.title('双 Y 轴示例')
    plt.savefig('/tmp/rsplot_example_twin.svg')
    print("已保存: /tmp/rsplot_example_twin.svg")


def example_log_scale():
    """对数坐标示例"""
    print("\n" + "=" * 60)
    print("对数坐标示例")
    print("=" * 60)
    
    x = np.linspace(0.1, 10, 100)
    y = np.exp(x)
    
    plt.figure()
    plt.semilogy(x, y, color='green')
    plt.xlabel('x')
    plt.ylabel('exp(x)')
    plt.title('半对数坐标')
    plt.grid(True)
    plt.savefig('/tmp/rsplot_example_log.svg')
    print("已保存: /tmp/rsplot_example_log.svg")


def example_imshow():
    """图像显示示例"""
    print("\n" + "=" * 60)
    print("图像显示示例")
    print("=" * 60)
    
    random.seed(42)
    data = random.rand(50, 50)
    
    plt.figure()
    plt.imshow(data, cmap='hot', aspect='auto')
    plt.title('热力图')
    plt.savefig('/tmp/rsplot_example_imshow.svg')
    print("已保存: /tmp/rsplot_example_imshow.svg")


def example_mixed():
    """混合图表示例"""
    print("\n" + "=" * 60)
    print("混合图表示例")
    print("=" * 60)
    
    x = np.linspace(0, 10, 100)
    
    plt.figure()
    plt.plot(x, np.sin(x), color='blue', label='sin')
    plt.scatter(x[::10], np.sin(x[::10]), color='red', s=20, label='points')
    plt.axhline(0, color='black', linestyle='--')
    plt.text(5, 0.5, 'sin(x)', fontsize=12, color='blue')
    plt.xlabel('x')
    plt.ylabel('y')
    plt.title('混合图表')
    plt.legend('upper right')
    plt.savefig('/tmp/rsplot_example_mixed.svg')
    print("已保存: /tmp/rsplot_example_mixed.svg")


def run_example(name):
    """运行指定示例"""
    examples = {
        'line': example_line_plot,
        'scatter': example_scatter,
        'bar': example_bar,
        'hist': example_histogram,
        'subplots': example_subplots,
        'pie': example_pie,
        'boxplot': example_boxplot,
        'errorbar': example_errorbar,
        'twin': example_twin_axis,
        'log': example_log_scale,
        'imshow': example_imshow,
        'mixed': example_mixed,
    }
    
    if name in examples:
        examples[name]()
    else:
        print(f"未知示例: {name}")
        print(f"可用示例: {list(examples.keys())}")


def run_all():
    """运行所有示例"""
    example_line_plot()
    example_scatter()
    example_bar()
    example_histogram()
    example_subplots()
    example_pie()
    example_boxplot()
    example_errorbar()
    example_twin_axis()
    example_log_scale()
    example_imshow()
    example_mixed()
    
    print("\n" + "=" * 60)
    print("所有示例运行完成!")
    print("图片已保存到 /tmp/rsplot_example_*.svg")
    print("=" * 60)


if __name__ == "__main__":
    run_all()
