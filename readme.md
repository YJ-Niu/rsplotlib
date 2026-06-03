项目介绍
本项目是由rust语言编写的python库，用于绘制2D图表。所有图表都基于matplotlib的API进行绘制。

1. 已使用uv创建python环境.venv，并已使用uv安装依赖matplotlib
2. 使用build_wheel.sh脚本构建pyhton库到.venv环境中
3. 可以使用uv pip安装.venv环境中的库
4. 需要结合rust plotters的特性进行开发， 不是一味地复制matplotlib的API

主要难点

1. 如何根据rust plotters的特性，实现matplotlib的API，并且绘图效果与matplotlib一致
2. svg格式和png格式的图片保存，效果和布局有区别

开发重点

1. 使用main.py绘制测试图表保存路径 plots/N238B W1-plots，与使用matplotlib 绘制的保存路径: N238B W1-plots 下的图片进行对比，保存格式为svg，可以已在main.py更改其他图片格式进行分析
2. 修复图片绘制的问题，确保与matplotlib绘制的图片效果和布局一致
   保存图片代码 plt.savefig(os.path.join(path_p, f'{plt_idix} {test_name}.svg'))

注意
不要在格式为svg上做太多的修改，否则其它图片格式（如png）会导致图片效果和布局与matplotlib不一致

修复好，进行重构，画图，对比，再修复问题，一次循环
