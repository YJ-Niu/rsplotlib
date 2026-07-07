use crate::core::colors::RgbColor;

/// 标注箭头参数（对应 matplotlib annotate 的 arrowprops）。
///
/// `arrowprops` 为 None 时不绘制箭头；非 None（哪怕空 dict）即绘制。
/// 两种模式：
/// - 「花式」：提供 `arrowstyle`（如 "->"、"<->"、"-|>"），按样式画杆 + 端点箭头；
/// - 「简单」：无 `arrowstyle`，用 `width`/`headwidth`/`headlength` 画一个实心渐变箭头。
#[derive(Clone)]
pub struct ArrowSpec {
    /// 归一化后的箭头样式（"-"/"->"/"<-"/"<->"/"-|>"/"<|-"/"<|-|>"/"simple"/"fancy"/"wedge"）。
    /// 空串表示未指定 arrowstyle（走「简单」实心箭头）。
    pub style: String,
    /// 描边 / 空心箭头颜色（来自 color / ec / edgecolor，回退到标注文本色）。
    pub color: String,
    /// 实心填充色（来自 facecolor / fc；None 时用 `color`）。
    pub face_color: Option<String>,
    /// 杆线宽（points）。
    pub linewidth: f64,
    /// 箭头头部尺寸缩放（points）；matplotlib 默认取文本字号。
    pub mutation_scale: f64,
    /// 文本一侧收缩量（points）。
    pub shrink_a: f64,
    /// 被标注点一侧收缩量（points）。
    pub shrink_b: f64,
    /// 「简单」箭头 `shrink`：从两端各收缩的总长度比例（0.0-0.5）。
    pub shrink_frac: f64,
    /// 透明度。
    pub alpha: f64,
    /// 「简单」箭头：杆宽（points）。
    pub width: f64,
    /// 「简单」箭头：头部底宽（points）。
    pub head_width: f64,
    /// 「简单」箭头：头部长度（points）。
    pub head_length: f64,
}

#[derive(Clone)]
pub enum PlotElement {
    Line {
        x: Vec<Option<f64>>,
        y: Vec<Option<f64>>,
        label: Option<String>,
        color: String,
        linestyle: String,
        marker: Option<String>,
        linewidth: f64,
        color_idx: usize,
        solid_capstyle: String,
        markersize: Option<f64>,
        markerfacecolor: Option<String>,
        markeredgecolor: Option<String>,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        s: f64,
        c: String,
        marker: String,
        label: Option<String>,
        alpha: f64,
        color_idx: usize,
        edgecolor: Option<String>,
        linewidth: Option<f64>,
    },
    ScatterMulti {
        x: Vec<f64>,
        y: Vec<f64>,
        s_list: Option<Vec<f64>>,
        c_list: Option<Vec<String>>,
        marker: String,
        label: Option<String>,
        alpha: f64,
        color_idx: usize,
        edgecolor: Option<String>,
        linewidth: Option<f64>,
    },
    Bar {
        x: Vec<f64>,
        height: Vec<f64>,
        width: f64,
        colors: Vec<String>,
        label: Option<String>,
        color_idx: usize,
    },
    BarH {
        y: Vec<f64>,
        width: Vec<f64>,
        height: f64,
        colors: Vec<String>,
        label: Option<String>,
        color_idx: usize,
    },
    Hist {
        /// 每个 dataset 的柱子几何: (pos_left, pos_right, val_base, val_top)
        /// pos = 分箱位置轴, val = 计数轴; 方向(横/竖)在渲染时交换坐标。
        bars: Vec<Vec<(f64, f64, f64, f64)>>,
        /// step/stepfilled 的轮廓折线, 每个 dataset 一条: (pos, val)
        outlines: Vec<Vec<(f64, f64)>>,
        histtype: String,
        orientation: String,
        label: Option<String>,
        alpha: f64,
        colors: Vec<String>,
        color_idx: usize,
    },
    Image {
        /// 逐像素已解析的 RGB（row-major）。origin 已在构建时应用：
        /// 绘制时第 0 行画在数据区底部，最后一行画在顶部。
        pixels: Vec<Vec<(u8, u8, u8)>>,
        /// 整体透明度（0.0-1.0）
        alpha: f64,
        /// 插值方法：`nearest`（块状、有分界线）或 `bilinear`/`bicubic`（平滑渐变）。
        interpolation: String,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        fontsize: f64,
        color: RgbColor,
        font_family: Option<String>,
    },
    HLine {
        y: f64,
        color: String,
        linestyle: String,
        linewidth: f64,
        color_idx: usize,
    },
    VLine {
        x: f64,
        color: String,
        linestyle: String,
        linewidth: f64,
        color_idx: usize,
    },
    Pie {
        x: Vec<f64>,
        labels: Option<Vec<String>>,
        colors: Option<Vec<String>>,
        autopct: Option<String>,
        startangle: f64,
        explode: Option<Vec<f64>>,
    },
    FillBetween {
        x: Vec<f64>,
        y1: Vec<f64>,
        y2: Vec<f64>,
        color: String,
        alpha: f64,
        label: Option<String>,
    },
    ErrorBar {
        x: Vec<f64>,
        y: Vec<f64>,
        yerr: Option<Vec<f64>>,
        xerr: Option<Vec<f64>>,
        fmt: String,
        color: String,
        label: Option<String>,
        capsize: f64,
    },
    Stem {
        x: Vec<f64>,
        y: Vec<f64>,
        linefmt: String,
        markerfmt: String,
        label: Option<String>,
    },
    Step {
        x: Vec<f64>,
        y: Vec<f64>,
        where_: String,
        label: Option<String>,
        color: String,
        linestyle: String,
        linewidth: f64,
    },
    BoxPlot {
        data: Vec<Vec<f64>>,
        labels: Option<Vec<String>>,
        vert: bool,
    },
    Annotate {
        text: String,
        xy: (f64, f64),
        xytext: Option<(f64, f64)>,
        fontsize: f64,
        color: String,
        /// 箭头参数；None 表示不画箭头（仅放置文本）。
        arrow: Option<ArrowSpec>,
    },
    Stack {
        x: Vec<f64>,
        y_series: Vec<Vec<f64>>,
        labels: Option<Vec<String>>,
        colors: Option<Vec<String>>,
        alpha: f64,
    },
    HSpan {
        y1: f64,
        y2: f64,
        color: String,
        alpha: f64,
    },
    VSpan {
        x1: f64,
        x2: f64,
        color: String,
        alpha: f64,
    },
    AxLine {
        xy1: (f64, f64),
        xy2: (f64, f64),
        color: String,
        linestyle: String,
        linewidth: f64,
    },
}
