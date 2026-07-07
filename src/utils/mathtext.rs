//! 二维数学排版（mathtext）渲染。
//!
//! Python 层 `pyplot._convert_math` 负责 `$...$` 检测、希腊字母/符号/重音/间距/
//! 字体命令的 Unicode 转换，并把**结构化构造**（上/下标、分式、二项式、genfrac、
//! 根号）编码为一段控制字符 IR 交给本模块。本模块把 IR 解析成盒模型树，用
//! plotters 的 `FontDesc::box_size` 度量字形宽度，计算二维布局（缩小并偏移的
//! 上下标、分子叠分母加分数线、根号加上盖线 vinculum），再输出多段文本 + 直线
//! 绘制原语，落到具体渲染站点（数据坐标的 chart / 像素坐标的 DrawingArea）。
//!
//! 纯文本（不含 IR）走快路径，仍以单次 `Text::new` 绘制，保持与既有渲染逐像素一致。
//!
//! IR 语法（须与 `python/rsplotlib/pyplot.py` 完全一致）：
//! - `\x02 s <base> \x1f <sup> \x1f <sub> \x03`  上/下标（sup、sub 为空表示无）
//! - `\x02 f <num> \x1f <den> \x03`              分式（带分数线）
//! - `\x02 b <num> \x1f <den> \x03`              二项式（括号，无线）
//! - `\x02 g <ld> \x1f <rd> \x1f <bar> \x1f <num> \x1f <den> \x03`  genfrac
//! - `\x02 r <index> \x1f <body> \x03`           根号（index 为空表示平方根）
//!
//! 字段内可再嵌套上述构造。

use plotters::coord::Shift;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::utils::font_stack;

const START: char = '\u{2}';
const SEP: char = '\u{1f}';
const END: char = '\u{3}';

/// 字符串是否包含数学 IR（含结构化构造）。不含则可走单行快路径。
pub fn contains_ir(s: &str) -> bool {
    s.contains(START)
}

/// 字符串是否包含上/下标构造（IR 中的 `START 's'`）。上标/下标使排版块比单行更高，
/// 供坐标轴标签据此增加离轴距离，避免上标/下标挤向刻度值与坐标轴。
pub fn has_script(s: &str) -> bool {
    s.contains("\u{2}s")
}

// ==================== 解析 ====================

enum Node {
    Row(Vec<Node>),
    Sym(String),
    Script {
        base: Box<Node>,
        sup: Option<Box<Node>>,
        sub: Option<Box<Node>>,
    },
    Frac {
        num: Box<Node>,
        den: Box<Node>,
        bar: bool,
        ldelim: String,
        rdelim: String,
    },
    Sqrt {
        index: Option<Box<Node>>,
        body: Box<Node>,
    },
}

impl Node {
    fn is_empty(&self) -> bool {
        match self {
            Node::Row(v) => v.iter().all(Node::is_empty),
            Node::Sym(s) => s.is_empty(),
            _ => false,
        }
    }
}

struct Parser {
    c: Vec<char>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    /// 读取一段序列直到遇到 `stops` 中的字符或字符串结束（不消费停止符）。
    fn parse_seq(&mut self, stops: &[char]) -> Node {
        let mut nodes: Vec<Node> = Vec::new();
        let mut buf = String::new();
        loop {
            match self.peek() {
                None => break,
                Some(ch) if stops.contains(&ch) => break,
                Some(c) if c == START => {
                    if !buf.is_empty() {
                        nodes.push(Node::Sym(std::mem::take(&mut buf)));
                    }
                    nodes.push(self.parse_construct());
                }
                Some(c) => {
                    buf.push(c);
                    self.i += 1;
                }
            }
        }
        if !buf.is_empty() {
            nodes.push(Node::Sym(buf));
        }
        if nodes.len() == 1 {
            nodes.pop().unwrap()
        } else {
            Node::Row(nodes)
        }
    }

    fn field(&mut self) -> Node {
        self.parse_seq(&[SEP, END])
    }

    fn eat(&mut self, ch: char) {
        if self.peek() == Some(ch) {
            self.i += 1;
        }
    }

    /// 当前位置为 START，解析一个结构化构造。
    fn parse_construct(&mut self) -> Node {
        self.i += 1; // 跳过 START
        let kind = self.peek().unwrap_or(END);
        self.i += 1; // 跳过类型字符
        let node = match kind {
            's' => {
                let base = self.field();
                self.eat(SEP);
                let sup = self.field();
                self.eat(SEP);
                let sub = self.field();
                Node::Script {
                    base: Box::new(base),
                    sup: opt(sup),
                    sub: opt(sub),
                }
            }
            'f' | 'b' => {
                let num = self.field();
                self.eat(SEP);
                let den = self.field();
                let (bar, ld, rd) = if kind == 'f' {
                    (true, String::new(), String::new())
                } else {
                    (false, "(".to_string(), ")".to_string())
                };
                Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    bar,
                    ldelim: ld,
                    rdelim: rd,
                }
            }
            'g' => {
                let ld = self.field();
                self.eat(SEP);
                let rd = self.field();
                self.eat(SEP);
                let bar = self.field();
                self.eat(SEP);
                let num = self.field();
                self.eat(SEP);
                let den = self.field();
                let bar_on =
                    !node_to_plain(&bar).trim().is_empty() && node_to_plain(&bar).trim() != "0";
                Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    bar: bar_on,
                    ldelim: node_to_plain(&ld),
                    rdelim: node_to_plain(&rd),
                }
            }
            'r' => {
                let index = self.field();
                self.eat(SEP);
                let body = self.field();
                Node::Sqrt {
                    index: opt(index),
                    body: Box::new(body),
                }
            }
            _ => Node::Sym(String::new()),
        };
        self.eat(END);
        node
    }
}

fn opt(n: Node) -> Option<Box<Node>> {
    if n.is_empty() {
        None
    } else {
        Some(Box::new(n))
    }
}

fn parse(s: &str) -> Node {
    let mut p = Parser {
        c: s.chars().collect(),
        i: 0,
    };
    p.parse_seq(&[])
}

// ==================== 单行 Unicode 降级（fallback） ====================

fn map_script(s: &str, sup: bool) -> String {
    s.chars()
        .map(|c| {
            let mapped = if sup { super_char(c) } else { sub_char(c) };
            // 仅当渲染字体确实含该上/下标字形时才采用 Unicode 形；否则退回普通字符，
            // 避免缺字形方框（如 DejaVu Sans 缺 U+1D62 下标 i）。
            match mapped {
                Some(m) if font_stack::char_supported(m) => m,
                _ => c,
            }
        })
        .collect()
}

fn super_char(c: char) -> Option<char> {
    Some(match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'a' => 'ᵃ',
        'b' => 'ᵇ',
        'c' => 'ᶜ',
        'd' => 'ᵈ',
        'e' => 'ᵉ',
        'f' => 'ᶠ',
        'g' => 'ᵍ',
        'h' => 'ʰ',
        'i' => 'ⁱ',
        'j' => 'ʲ',
        'k' => 'ᵏ',
        'l' => 'ˡ',
        'm' => 'ᵐ',
        'n' => 'ⁿ',
        'o' => 'ᵒ',
        'p' => 'ᵖ',
        'r' => 'ʳ',
        's' => 'ˢ',
        't' => 'ᵗ',
        'u' => 'ᵘ',
        'v' => 'ᵛ',
        'w' => 'ʷ',
        'x' => 'ˣ',
        'y' => 'ʸ',
        'z' => 'ᶻ',
        _ => return None,
    })
}

fn sub_char(c: char) -> Option<char> {
    Some(match c {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '-' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'h' => 'ₕ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'k' => 'ₖ',
        'l' => 'ₗ',
        'm' => 'ₘ',
        'n' => 'ₙ',
        'o' => 'ₒ',
        'p' => 'ₚ',
        'r' => 'ᵣ',
        's' => 'ₛ',
        't' => 'ₜ',
        'u' => 'ᵤ',
        'v' => 'ᵥ',
        'x' => 'ₓ',
        _ => return None,
    })
}

fn node_to_plain(n: &Node) -> String {
    let mut out = String::new();
    plain_into(n, &mut out);
    out
}

/// 该节点是否为大型（n 元）算符——上/下标应堆叠在符号上下方而非置于右侧。
fn is_bigop(n: &Node) -> bool {
    matches!(
        node_to_plain(n).trim(),
        "∑" | "∏" | "∐" | "⋀" | "⋁" | "⋂" | "⋃" | "⨆" | "⨅" | "⨁" | "⨂" | "⨀"
    )
}

fn plain_into(n: &Node, out: &mut String) {
    match n {
        Node::Row(v) => {
            for c in v {
                plain_into(c, out);
            }
        }
        Node::Sym(s) => out.push_str(s),
        Node::Script { base, sup, sub } => {
            plain_into(base, out);
            if let Some(sub) = sub {
                out.push_str(&map_script(&node_to_plain(sub), false));
            }
            if let Some(sup) = sup {
                out.push_str(&map_script(&node_to_plain(sup), true));
            }
        }
        Node::Frac {
            num,
            den,
            bar,
            ldelim,
            rdelim,
        } => {
            out.push_str(ldelim);
            plain_into(num, out);
            out.push(if *bar { '/' } else { ' ' });
            plain_into(den, out);
            out.push_str(rdelim);
        }
        Node::Sqrt { index, body } => {
            if let Some(index) = index {
                out.push_str(&map_script(&node_to_plain(index), true));
            }
            out.push('√');
            plain_into(body, out);
        }
    }
}

/// 把可能含 IR 的字符串降级为单行 Unicode 近似（供 plotters 内置绘制的站点使用：
/// 居中的 x/y 轴标签、suptitle、类别刻度标签等无法承载二维排版）。
pub fn to_plain(s: &str) -> String {
    if !contains_ir(s) {
        return s.to_string();
    }
    node_to_plain(&parse(s))
}

// ==================== 布局 ====================

// 布局常量（相对字号 em 的比例），按视觉效果调优。
const SCRIPT_SCALE: f64 = 0.7; // 大型算符上/下极限（∑ ∏ 上下方）的缩放
const SIDE_SCRIPT_SCALE: f64 = 0.49; // 右侧上/下标缩放（比极限再小 30%）
const LEAF_UP: f64 = 0.42; // 叶子字形中心线以上视觉半高
const LEAF_DOWN: f64 = 0.30; // 叶子字形中心线以下视觉半高
const SUP_SHIFT: f64 = 0.44; // 上标中心相对基线中心上移
const SUB_SHIFT: f64 = 0.34; // 下标中心相对基线中心下移
const SCRIPT_KERN: f64 = 0.04; // 基字符与上下标之间的水平间隙
const FRAC_GAP: f64 = 0.12; // 分子/分母与分数线的间距
const FRAC_PAD: f64 = 0.12; // 分式左右内边距
const FRAC_AXIS: f64 = 0.22; // 分数线相对中心线的上移量（对齐相邻文字的数学轴）
const FRAC_BAR: f64 = 0.05; // 分数线厚度（相对字号）
const FRAC_INK: f64 = 0.18; // 分子/分母整体下移补偿（抵消字体盒居中偏移），使横线居中
const DELIM_FILL: f64 = 0.8; // 定界符字号 = 内容高度 / 此值（越小括号越大、越能包住内容）
const SQRT_GAP: f64 = 0.42; // 根号盖线与被开方内容的间距
const LIMIT_GAP: f64 = 0.05; // 大型算符（∑ ∏ 等）上/下标与符号之间的竖直间距

struct Run {
    text: String,
    dx: f64, // 左边缘 x（相对布局左边缘）
    dy: f64, // 中心 y（相对中心线，负为上）
    size: f64,
}

struct Rule {
    x0: f64,
    x1: f64,
    y: f64, // 中心线相对 y
    thick: f64,
}

/// 一个节点的布局结果：坐标系以左边缘为 x=0、数学中心线为 y=0（负值向上）。
struct Layout {
    runs: Vec<Run>,
    rules: Vec<Rule>,
    width: f64,
    up: f64,   // 中心线以上高度
    down: f64, // 中心线以下高度
}

fn text_width(s: &str, family: &str, size: f64) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let font: FontDesc = (family, size).into();
    font.box_size(s)
        .map(|(w, _)| w as f64)
        .unwrap_or(0.55 * size * s.chars().count() as f64)
}

fn layout(node: &Node, size: f64, family: &str) -> Layout {
    match node {
        Node::Sym(s) => {
            let w = text_width(s, family, size);
            Layout {
                runs: vec![Run {
                    text: s.clone(),
                    dx: 0.0,
                    dy: 0.0,
                    size,
                }],
                rules: Vec::new(),
                width: w,
                up: LEAF_UP * size,
                down: LEAF_DOWN * size,
            }
        }
        Node::Row(children) => {
            let mut runs = Vec::new();
            let mut rules = Vec::new();
            let mut x = 0.0;
            let mut up: f64 = 0.0;
            let mut down: f64 = 0.0;
            for ch in children {
                let l = layout(ch, size, family);
                for r in l.runs {
                    runs.push(Run {
                        text: r.text,
                        dx: r.dx + x,
                        dy: r.dy,
                        size: r.size,
                    });
                }
                for r in l.rules {
                    rules.push(Rule {
                        x0: r.x0 + x,
                        x1: r.x1 + x,
                        y: r.y,
                        thick: r.thick,
                    });
                }
                x += l.width;
                up = up.max(l.up);
                down = down.max(l.down);
            }
            Layout {
                runs,
                rules,
                width: x,
                up,
                down,
            }
        }
        Node::Script { base, sup, sub } => {
            let b = layout(base, size, family);
            let ssize = SCRIPT_SCALE * size;
            // 大型算符（∑ ∏ …）：上标居中堆在符号上方、下标居中堆在符号下方。
            if is_bigop(base) && (sup.is_some() || sub.is_some()) {
                let gap = LIMIT_GAP * size;
                let sup_l = sup.as_ref().map(|n| layout(n, ssize, family));
                let sub_l = sub.as_ref().map(|n| layout(n, ssize, family));
                let sup_w = sup_l.as_ref().map(|l| l.width).unwrap_or(0.0);
                let sub_w = sub_l.as_ref().map(|l| l.width).unwrap_or(0.0);
                let total_w = b.width.max(sup_w).max(sub_w);
                let mut runs = Vec::new();
                let mut rules = Vec::new();
                let bx = (total_w - b.width) / 2.0;
                for r in b.runs {
                    runs.push(Run {
                        text: r.text,
                        dx: r.dx + bx,
                        dy: r.dy,
                        size: r.size,
                    });
                }
                for r in b.rules {
                    rules.push(Rule {
                        x0: r.x0 + bx,
                        x1: r.x1 + bx,
                        y: r.y,
                        thick: r.thick,
                    });
                }
                let mut up = b.up;
                let mut down = b.down;
                if let Some(s) = sup_l {
                    let sx = (total_w - s.width) / 2.0;
                    let cy = -(b.up + gap + s.down);
                    for r in s.runs {
                        runs.push(Run {
                            text: r.text,
                            dx: r.dx + sx,
                            dy: r.dy + cy,
                            size: r.size,
                        });
                    }
                    for r in s.rules {
                        rules.push(Rule {
                            x0: r.x0 + sx,
                            x1: r.x1 + sx,
                            y: r.y + cy,
                            thick: r.thick,
                        });
                    }
                    up = up.max(-cy + s.up);
                }
                if let Some(s) = sub_l {
                    let sx = (total_w - s.width) / 2.0;
                    let cy = b.down + gap + s.up;
                    for r in s.runs {
                        runs.push(Run {
                            text: r.text,
                            dx: r.dx + sx,
                            dy: r.dy + cy,
                            size: r.size,
                        });
                    }
                    for r in s.rules {
                        rules.push(Rule {
                            x0: r.x0 + sx,
                            x1: r.x1 + sx,
                            y: r.y + cy,
                            thick: r.thick,
                        });
                    }
                    down = down.max(cy + s.down);
                }
                return Layout {
                    runs,
                    rules,
                    width: total_w,
                    up,
                    down,
                };
            }
            let mut runs = b.runs;
            let mut rules = b.rules;
            let mut up = b.up;
            let mut down = b.down;
            let sx = b.width + SCRIPT_KERN * size;
            let side_size = SIDE_SCRIPT_SCALE * size;
            let mut script_w: f64 = 0.0;
            if let Some(sup) = sup {
                let s = layout(sup, side_size, family);
                let cy = -SUP_SHIFT * size;
                for r in s.runs {
                    runs.push(Run {
                        text: r.text,
                        dx: r.dx + sx,
                        dy: r.dy + cy,
                        size: r.size,
                    });
                }
                for r in s.rules {
                    rules.push(Rule {
                        x0: r.x0 + sx,
                        x1: r.x1 + sx,
                        y: r.y + cy,
                        thick: r.thick,
                    });
                }
                up = up.max(-cy + s.up);
                script_w = script_w.max(s.width);
            }
            if let Some(sub) = sub {
                let s = layout(sub, side_size, family);
                let cy = SUB_SHIFT * size;
                for r in s.runs {
                    runs.push(Run {
                        text: r.text,
                        dx: r.dx + sx,
                        dy: r.dy + cy,
                        size: r.size,
                    });
                }
                for r in s.rules {
                    rules.push(Rule {
                        x0: r.x0 + sx,
                        x1: r.x1 + sx,
                        y: r.y + cy,
                        thick: r.thick,
                    });
                }
                down = down.max(cy + s.down);
                script_w = script_w.max(s.width);
            }
            Layout {
                runs,
                rules,
                width: sx + script_w,
                up,
                down,
            }
        }
        Node::Frac {
            num,
            den,
            bar,
            ldelim,
            rdelim,
        } => layout_frac(num, den, *bar, ldelim, rdelim, size, family),
        Node::Sqrt { index, body } => layout_sqrt(index.as_deref(), body, size, family),
    }
}

#[allow(clippy::borrowed_box)]
fn layout_frac(
    num: &Node,
    den: &Node,
    bar: bool,
    ldelim: &str,
    rdelim: &str,
    size: f64,
    family: &str,
) -> Layout {
    let n = layout(num, size, family);
    let d = layout(den, size, family);
    let inner = n.width.max(d.width);
    let pad = FRAC_PAD * size;
    let fw = inner + 2.0 * pad;
    let bar_thick = (FRAC_BAR * size).max(1.0);
    let gap = FRAC_GAP * size;
    // 分数线上移 FRAC_AXIS（数学轴），使其对齐相邻文字视觉中线，而非落在偏低的行中线。
    let axis = FRAC_AXIS * size;
    let bar_y = -axis;
    // 字形以 VPos::Center 绘制时，plotters 居中的是整个字体盒（含 descent 空白），墨迹
    // 视觉中心比给定 dy 偏高；而分数线按精确 y 绘制，于是相对偏低、贴向分母。将分子分母
    // 整体下移 FRAC_INK 抵消该偏移，使墨迹落回模型位置、横线居中于两者之间。
    let ink = FRAC_INK * size;
    let num_cy = bar_y - (bar_thick * 0.5 + gap + n.down) + ink;
    let den_cy = bar_y + bar_thick * 0.5 + gap + d.up + ink;

    let mut runs = Vec::new();
    let mut rules = Vec::new();
    let nx = pad + (inner - n.width) / 2.0;
    for r in n.runs {
        runs.push(Run {
            text: r.text,
            dx: r.dx + nx,
            dy: r.dy + num_cy,
            size: r.size,
        });
    }
    for r in n.rules {
        rules.push(Rule {
            x0: r.x0 + nx,
            x1: r.x1 + nx,
            y: r.y + num_cy,
            thick: r.thick,
        });
    }
    let dx = pad + (inner - d.width) / 2.0;
    for r in d.runs {
        runs.push(Run {
            text: r.text,
            dx: r.dx + dx,
            dy: r.dy + den_cy,
            size: r.size,
        });
    }
    for r in d.rules {
        rules.push(Rule {
            x0: r.x0 + dx,
            x1: r.x1 + dx,
            y: r.y + den_cy,
            thick: r.thick,
        });
    }
    if bar {
        rules.push(Rule {
            x0: pad * 0.5,
            x1: fw - pad * 0.5,
            y: bar_y,
            thick: bar_thick,
        });
    }
    let up = -num_cy + n.up;
    let down = den_cy + d.down;

    let mut out = Layout {
        runs,
        rules,
        width: fw,
        up,
        down,
    };
    if !ldelim.is_empty() || !rdelim.is_empty() {
        wrap_delims(&mut out, ldelim, rdelim, family);
    }
    out
}

/// 用放大的定界符字形（如 `(` `)`）把分式/二项式包起来。字号由内容高度决定
/// （DELIM_FILL 越小括号越大），竖直方向以内容盒中心对齐。
fn wrap_delims(inner: &mut Layout, ldelim: &str, rdelim: &str, family: &str) {
    let h = inner.up + inner.down;
    // 定界符字号由内容高度决定（DELIM_FILL 越小括号越大）。
    let dsize = (h / DELIM_FILL * 0.8).max(1.0);
    // 竖直中心：内容盒中心 (down-up)/2，随分数上移一并抬升。
    let cy = (inner.down - inner.up) / 2.0;
    // 定界符字形微调：向右、向下各偏移括号大小的 5%。
    let nudge = 0.05 * dsize;
    let lw = if ldelim.is_empty() {
        0.0
    } else {
        text_width(ldelim, family, dsize)
    };
    let rw = if rdelim.is_empty() {
        0.0
    } else {
        text_width(rdelim, family, dsize)
    };
    // 内容整体右移 lw，为左定界符腾出空间。
    for r in inner.runs.iter_mut() {
        r.dx += lw;
    }
    for r in inner.rules.iter_mut() {
        r.x0 += lw;
        r.x1 += lw;
    }
    if !ldelim.is_empty() {
        inner.runs.push(Run {
            text: ldelim.to_string(),
            dx: nudge,
            dy: cy + nudge * 1.1,
            size: dsize,
        });
    }
    if !rdelim.is_empty() {
        inner.runs.push(Run {
            text: rdelim.to_string(),
            dx: lw + inner.width * 0.9 + nudge,
            dy: cy + nudge * 1.1,
            size: dsize,
        });
    }
    inner.width += lw + rw;
}

#[allow(clippy::borrowed_box)]
fn layout_sqrt(index: Option<&Node>, body: &Node, size: f64, family: &str) -> Layout {
    let b = layout(body, size, family);
    let vth = (0.05 * size).max(1.0);
    let body_h = b.up + b.down;
    // 根号字形放大以贴近内容高度。
    let rad_size = (body_h / 0.72).max(size);
    let rad_w = text_width("√", family, rad_size);
    // 盖线与内容的间距随根号字形大小缩放：√ 越大其顶端越高，盖线也应相应上移，
    // 才能与放大的根号符号顶端衔接（用 size 会使大根号的盖线偏低、贴住内容）。
    let gap = SQRT_GAP * rad_size * 0.77;

    // n 次根：先布局指数，为其在根号左侧预留宽度（指数与根号斜线部分重叠，
    // 因此只预留其大部分宽度，而非全宽）。
    let (index_layout, left_pad) = match index {
        Some(idx) => {
            let isize = 0.55 * size;
            let il = layout(idx, isize, family);
            let pad = (il.width - rad_w * 0.35).max(il.width * 0.5);
            (Some(il), pad)
        }
        None => (None, 0.0),
    };

    let mut runs = Vec::new();
    let mut rules = Vec::new();
    // 根号符号与被开方内容整体右移 left_pad（为左上方指数腾出空间）。
    runs.push(Run {
        text: "√".to_string(),
        dx: left_pad,
        dy: 0.0,
        size: rad_size,
    });
    let bx = left_pad + rad_w;
    for r in b.runs {
        runs.push(Run {
            text: r.text,
            dx: r.dx + bx,
            dy: r.dy,
            size: r.size,
        });
    }
    for r in b.rules {
        rules.push(Rule {
            x0: r.x0 + bx,
            x1: r.x1 + bx,
            y: r.y,
            thick: r.thick,
        });
    }
    let vin_y = -(b.up + gap);
    rules.push(Rule {
        x0: bx * 1.1,
        x1: bx + b.width * 1.1,
        y: vin_y,
        thick: vth,
    });
    let mut up = b.up + gap + vth;
    let down = b.down.max(rad_size * LEAF_DOWN);

    // 指数放在根号左上方。
    if let Some(il) = index_layout {
        let iy = -(b.up * 0.5 + gap);
        for r in il.runs {
            runs.push(Run {
                text: r.text,
                dx: r.dx,
                dy: r.dy + iy,
                size: r.size,
            });
        }
        for r in il.rules {
            rules.push(Rule {
                x0: r.x0,
                x1: r.x1,
                y: r.y + iy,
                thick: r.thick,
            });
        }
        up = up.max(-iy + il.up);
    }

    Layout {
        runs,
        rules,
        width: left_pad + rad_w + b.width,
        up,
        down,
    }
}

// ==================== 对齐与绘制 ====================

#[derive(Clone, Copy)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
    Baseline,
}

/// 计算把布局锚定到 (0,0) 时的整体偏移：返回 (ox, oy)，使各 run 的最终屏幕偏移
/// 为 (ox + run.dx, oy + run.dy)。run 以 Pos(Left, Center) 绘制。
fn align_offset(l: &Layout, h: HAlign, v: VAlign) -> (f64, f64) {
    let ox = match h {
        HAlign::Left => 0.0,
        HAlign::Center => -l.width / 2.0,
        HAlign::Right => -l.width,
    };
    let oy = match v {
        VAlign::Center => 0.0,
        VAlign::Top => l.up,
        VAlign::Bottom => -l.down,
        // 近似基线：中心线略高于基线，基线约在中心线下 0.30em（以整体 up 估算字号）。
        VAlign::Baseline => -l.down * 0.25,
    };
    (ox, oy)
}

fn plotters_pos(h: HAlign, v: VAlign) -> Pos {
    let hp = match h {
        HAlign::Left => HPos::Left,
        HAlign::Center => HPos::Center,
        HAlign::Right => HPos::Right,
    };
    let vp = match v {
        VAlign::Top => VPos::Top,
        VAlign::Center => VPos::Center,
        VAlign::Bottom | VAlign::Baseline => VPos::Bottom,
    };
    Pos::new(hp, vp)
}

fn rule_stroke(thick: f64) -> u32 {
    ((thick).round() as i64).max(1) as u32
}

/// 在 chart（数据坐标）上绘制可能含数学 IR 的文本。纯文本走单行快路径。
#[allow(clippy::too_many_arguments)]
pub fn draw_math_chart<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    x: f64,
    y: f64,
    s: &str,
    size: f64,
    color: RGBColor,
    family: Option<&str>,
    h: HAlign,
    v: VAlign,
    dy_px: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let plain = to_plain(s);
    let fam = font_stack::resolve_font_family(&plain, family);
    let nudge = dy_px.round() as i32;
    if !contains_ir(s) {
        let style = FontDesc::from((fam.as_str(), size))
            .color(&color)
            .pos(plotters_pos(h, v));
        chart
            .draw_series(std::iter::once(
                plotters::element::EmptyElement::at((x, y))
                    + plotters::element::Text::new(s.to_string(), (0, nudge), style),
            ))
            .map_err(|e| PyRuntimeError::new_err(format!("math text: {}", e)))?;
        return Ok(());
    }
    let l = layout(&parse(s), size, &fam);
    let (ox, oy) = align_offset(&l, h, v);
    for r in &l.runs {
        let style = FontDesc::from((fam.as_str(), r.size))
            .color(&color)
            .pos(Pos::new(HPos::Left, VPos::Center));
        let dx = (ox + r.dx).round() as i32;
        let dy = (oy + r.dy).round() as i32 + nudge;
        chart
            .draw_series(std::iter::once(
                plotters::element::EmptyElement::at((x, y))
                    + plotters::element::Text::new(r.text.clone(), (dx, dy), style),
            ))
            .map_err(|e| PyRuntimeError::new_err(format!("math run: {}", e)))?;
    }
    for r in &l.rules {
        let style = color.stroke_width(rule_stroke(r.thick));
        let x0 = (ox + r.x0).round() as i32;
        let x1 = (ox + r.x1).round() as i32;
        let yy = (oy + r.y).round() as i32 + nudge;
        chart
            .draw_series(std::iter::once(
                plotters::element::EmptyElement::at((x, y))
                    + PathElement::new(vec![(x0, yy), (x1, yy)], style),
            ))
            .map_err(|e| PyRuntimeError::new_err(format!("math rule: {}", e)))?;
    }
    Ok(())
}

/// 在像素坐标的 DrawingArea 上绘制可能含数学 IR 的文本。纯文本走单行快路径。
#[allow(clippy::too_many_arguments)]
pub fn draw_math_area<DB: DrawingBackend>(
    area: &DrawingArea<DB, Shift>,
    x: f64,
    y: f64,
    s: &str,
    size: f64,
    color: RGBColor,
    family: Option<&str>,
    h: HAlign,
    v: VAlign,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let plain = to_plain(s);
    let fam = font_stack::resolve_font_family(&plain, family);
    if !contains_ir(s) {
        let style = FontDesc::from((fam.as_str(), size))
            .color(&color)
            .pos(plotters_pos(h, v));
        area.draw_text(s, &style, (x.round() as i32, y.round() as i32))
            .map_err(|e| PyRuntimeError::new_err(format!("math text: {}", e)))?;
        return Ok(());
    }
    let l = layout(&parse(s), size, &fam);
    let (ox, oy) = align_offset(&l, h, v);
    for r in &l.runs {
        let style = FontDesc::from((fam.as_str(), r.size))
            .color(&color)
            .pos(Pos::new(HPos::Left, VPos::Center));
        let px = (x + ox + r.dx).round() as i32;
        let py = (y + oy + r.dy).round() as i32;
        area.draw_text(&r.text, &style, (px, py))
            .map_err(|e| PyRuntimeError::new_err(format!("math run: {}", e)))?;
    }
    for r in &l.rules {
        let style = color.stroke_width(rule_stroke(r.thick));
        let x0 = (x + ox + r.x0).round() as i32;
        let x1 = (x + ox + r.x1).round() as i32;
        let yy = (y + oy + r.y).round() as i32;
        area.draw(&PathElement::new(vec![(x0, yy), (x1, yy)], style))
            .map_err(|e| PyRuntimeError::new_err(format!("math rule: {}", e)))?;
    }
    Ok(())
}

/// 在像素坐标 DrawingArea 上，以 270° 旋转（自下而上阅读，用于 y 轴标签）绘制可能含
/// 数学 IR 的文本，实现二维排版（真实上/下标、分式线、根号盖线）。
///
/// 坐标约定（屏幕 y 向下）：把水平布局整体旋转 90°（逆时针）——布局的「向右阅读」方向
/// 映射到屏幕「向上」，布局的「向下」方向映射到屏幕「向右」。因此：
/// - `anchor_x`：y 标签区左缘的屏幕 x；文字块自此向右延伸（朝向数据区），
///   宽度为内容的 up+down。
/// - `anchor_y` 与 `valign` 决定阅读方向（屏幕竖直）上的对齐：
///   - `Center`：阅读中点对齐 `anchor_y`；
///   - `Top`：阅读终点（块顶）落在 `anchor_y`，块向下延伸（loc="top"）；
///   - `Bottom`：阅读起点（块底）落在 `anchor_y`，块向上延伸（loc="bottom"）。
#[allow(clippy::too_many_arguments)]
pub fn draw_math_area_rotated<DB: DrawingBackend>(
    area: &DrawingArea<DB, Shift>,
    anchor_x: f64,
    anchor_y: f64,
    s: &str,
    size: f64,
    color: RGBColor,
    family: Option<&str>,
    valign: VAlign,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let plain = to_plain(s);
    let fam = font_stack::resolve_font_family(&plain, family);
    let l = layout(&parse(s), size, &fam);
    let w = l.width;
    // 垂直（perp）方向：布局最左 (ly=-up) 落在 anchor_x，块向右延伸 up+down。
    let perp = l.up;
    // 阅读方向偏移：使块按 valign 对齐到 anchor_y。
    let read = match valign {
        VAlign::Center => -w / 2.0,
        VAlign::Top => -w,
        VAlign::Bottom | VAlign::Baseline => 0.0,
    };
    // 布局点 (lx, ly) → 屏幕 (anchor_x + ly + perp, anchor_y - lx - read)。
    for r in &l.runs {
        let style = FontDesc::from((fam.as_str(), r.size))
            .color(&color)
            .transform(FontTransform::Rotate270)
            .pos(Pos::new(HPos::Left, VPos::Center));
        let sx = (anchor_x + r.dy + perp).round() as i32;
        let sy = (anchor_y - r.dx - read).round() as i32;
        area.draw_text(&r.text, &style, (sx, sy))
            .map_err(|e| PyRuntimeError::new_err(format!("math run: {}", e)))?;
    }
    for r in &l.rules {
        let style = color.stroke_width(rule_stroke(r.thick));
        let sx = (anchor_x + r.y + perp).round() as i32;
        let sy0 = (anchor_y - r.x0 - read).round() as i32;
        let sy1 = (anchor_y - r.x1 - read).round() as i32;
        area.draw(&PathElement::new(vec![(sx, sy0), (sx, sy1)], style))
            .map_err(|e| PyRuntimeError::new_err(format!("math rule: {}", e)))?;
    }
    Ok(())
}
