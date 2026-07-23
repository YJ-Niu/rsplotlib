"""rsplotlib.pyplot - Matplotlib pyplot 兼容接口

此模块提供与 matplotlib.pyplot 兼容的 API，所有函数代理到 rsplotlib 核心模块。
使用方法: import rsplotlib.pyplot as plt
"""

import math
from . import rsplotlib as _rsplotlib
from .figure._defaults import DEFAULT_FIGSIZE
# ============ 样式接口 ============
from .utils import style as _style_module
from .pylab import mpl

# 延迟获取 mpl.rcParams，避免 pyplot <-> pylab 循环导入


def _get_rcparams():
    """从 pylab.mpl 获取 rcParams，统一配置入口"""
    return mpl.rcParams


# 模块级 rcParams，与 mpl.rcParams 为同一单例，供 plt.rcParams 使用
rcParams = mpl.rcParams


def rc(group, **kwargs):
    """设置全局 rcParams（matplotlib.rc 兼容）。

    - ``rc('lines', linewidth=2, color='r')`` 等价于设置
      ``rcParams['lines.linewidth'] = 2`` 与 ``rcParams['lines.color'] = 'r'``。
    - ``group`` 也可为分组名的列表/元组，如 ``('xtick', 'ytick')``。
    - 传入 dict 时按 ``{完整键: 值}`` 直接更新 rcParams（空 dict 为空操作）。
    """
    if isinstance(group, dict):
        for key, value in group.items():
            rcParams[key] = value
        return
    if isinstance(group, str):
        group = (group,)
    for g in group:
        for name, value in kwargs.items():
            rcParams[f'{g}.{name}'] = value


# ==================== 内部辅助函数 ====================

def _to_list(obj):
    """将数组对象或其他可迭代对象转换为 Python list

    支持带 tolist() 方法的数组对象、Python list、tuple 及其他可迭代对象。
    标量值直接返回。
    """
    if obj is None:
        return None
    # 数组对象（带 tolist 方法）
    if hasattr(obj, 'tolist'):
        return obj.tolist()
    # Python list/tuple 或其他可迭代对象
    if isinstance(obj, (list, tuple)):
        return list(obj)
    # 标量
    return obj


def _to_list_recursive(obj):
    """递归转换嵌套的数组对象为 Python list"""
    if obj is None:
        return None
    if hasattr(obj, 'tolist'):
        return obj.tolist()
    if isinstance(obj, (list, tuple)):
        return [_to_list_recursive(item) for item in obj]
    return obj


def _buffer_kind(obj):
    """返回 obj 的 dtype kind 字符（numpy 约定 'f'/'i'/'u'/'b'/'M'/'S'/'U'/'c'…），
    无法判定时返回 None。

    优先读廉价的 obj.dtype.kind：第三方数组库的 __array_interface__ 属性每次访问都会即时把
    整个缓冲区序列化成 bytes（百万点约 2.6ms/次，随后被丢弃），而 dtype 访问是 O(1)。
    仅当对象没有 dtype（如 Python list、标量、或仅实现数组接口的第三方缓冲）
    时，才回退读取 __array_interface__ 的 typestr。
    """
    dt = None
    try:
        dt = getattr(obj, 'dtype', None)
    except Exception:
        # 第三方数组库对 datetime64[h]/timedelta64 等 dtype 的 .dtype 属性会抛 TypeError
        # （非 AttributeError，getattr 默认值挡不住）。此处按"无法判定"处理，落到
        # __array_interface__ 回退——这些非数值 dtype 本就应返回非 fiub kind。
        dt = None
    if dt is not None:
        kind = getattr(dt, 'kind', None)
        if isinstance(kind, str) and len(kind) == 1:
            return kind
    ai = getattr(obj, '__array_interface__', None)
    if isinstance(ai, dict):
        typestr = ai.get('typestr', '')
        return typestr[1:2] if len(typestr) >= 2 else None
    return None


def _is_numeric_buffer(obj):
    """obj 是否为纯数值缓冲数组（float/int/uint/bool）。

    这类数组可零拷贝下沉给 Rust，且不可能是字符串类别或日期，故可跳过 .tolist()
    全量物化（plot/scatter 热路径最大收益）。dtype kind 为 datetime64('M')、字符串
    ('U'/'S')、复数('c')、timedelta('m')、object('O') 均需 Python 侧特殊处理，返回
    False 交回原有 .tolist() 路径。
    """
    return _buffer_kind(obj) in ('f', 'i', 'u', 'b')


def _numeric_buffer_1d_len(obj):
    """若 obj 为一维纯数值缓冲数组 (float/int/uint/bool)，返回其长度；否则 None。

    用于 scatter 数值 c 的快路径：一维数值缓冲不可能是颜色字符串或 RGB(A) 行数组，
    可零拷贝直传 Rust 经 colormap 上色，跳过 .tolist() / 逐元素类别检查 / [float(v)] 物化。
    """
    if _buffer_kind(obj) not in ('f', 'i', 'u', 'b'):
        return None
    shape = getattr(obj, 'shape', None)
    if shape is None:
        ai = getattr(obj, '__array_interface__', None)
        if isinstance(ai, dict):
            shape = ai.get('shape')
    if not (isinstance(shape, tuple) and len(shape) == 1):
        return None
    return shape[0]


def _reduce_to_float(v):
    """把数组规约结果 (.min()/.max()) 归一为 Python float。

    数组库的规约返回类型可能不稳定：有时返回 Python float，
    有时返回 0 维 ndarray（float() 报错，但 .item() 可取标量）。
    """
    if isinstance(v, (int, float)):
        return float(v)
    item = getattr(v, 'item', None)
    if item is not None:
        return float(item())
    return float(v)


def _to_seq(obj):
    """一维数值参数的透传辅助：纯数值缓冲数组原样返回，交由 Rust 层零拷贝读取
    原始缓冲区（避免 .tolist() 生成大量 Python 对象）；其余对象（含 datetime64/
    字符串数组、Python list/tuple、标量）走 _to_list 保持原行为。

    仅用于纯数值、直接下沉给 Rust 的坐标/长度参数，不用于需在 Python 侧
    做类别检测或逐元素判断的参数。
    """
    if _is_numeric_buffer(obj):
        return obj
    return _to_list(obj)


def _replace_from_data(value, data):
    """matplotlib `data` 关键字支持：当 data 提供且 value 为字符串键时，
    以 data[value] 替换。查找失败（非键 / data 不支持索引）时保持原值，
    这样普通颜色字符串（如 c='red'）不会被误当作数据键。
    """
    if data is not None and isinstance(value, str):
        try:
            return data[value]
        except (KeyError, TypeError, IndexError):
            return value
    return value


def _categorical(vals):
    """分类坐标支持：若 vals 是含字符串的序列，返回 (等距整数位置, 标签列表)；
    否则原样返回 (vals, None)。与 matplotlib 一致——字符串映射到 0,1,2,... 位置，
    字符串本身作为该轴刻度标签。

    性能：数值缓冲区数组 (纯 float/int/bool) 不可能含字符串，直接原样返回，
    避免 .tolist() 把上百万元素物化成 Python 对象 (plot/scatter 的热路径)。字符串/
    datetime64 数组的 typestr kind 不是数值，_is_numeric_buffer 返回 False，才走 tolist
    做类别检测。
    """
    if vals is None:
        return vals, None
    if _is_numeric_buffer(vals):
        return vals, None
    if isinstance(vals, (list, tuple)):
        seq = vals
    elif hasattr(vals, 'tolist'):
        seq = vals.tolist()
    else:
        return vals, None
    if any(isinstance(v, str) for v in seq):
        labels = [str(v) for v in seq]
        return list(range(len(labels))), labels
    return vals, None


def _maybe_dates_to_num(seq):
    """日期坐标支持：若 seq 为 datetime/date 序列 (含 numpy datetime64 数组，
    经 .tolist() 得 datetime 对象)，转为自 1970-01-01 起天数的 float 列表；
    否则返回 None。与 matplotlib 日期约定一致，供 ConciseDateFormatter 反解。

    性能：纯数值缓冲区数组直接返回 None，避免 .tolist() 物化 (仅为窥视首元素)。
    datetime64 数组 kind 为 'M'，非数值缓冲，仍走 .tolist() 做日期转换。
    """
    import datetime as _dt
    if seq is None or isinstance(seq, (str, _dt.date, _dt.datetime)):
        return None
    if _is_numeric_buffer(seq):
        return None
    if isinstance(seq, (list, tuple)):
        lst = seq
    elif hasattr(seq, 'tolist'):
        lst = seq.tolist()
    else:
        return None
    if len(lst) == 0 or not isinstance(lst[0], (_dt.date, _dt.datetime)):
        return None
    epoch = _dt.datetime(1970, 1, 1)
    out = []
    for v in lst:
        if isinstance(v, _dt.datetime):
            out.append((v - epoch).total_seconds() / 86400.0)
        elif isinstance(v, _dt.date):
            out.append((_dt.datetime(v.year, v.month, v.day) - epoch).total_seconds() / 86400.0)
        else:
            return None
    return out


def _is_scatter_sequence(obj):
    """判断是否为序列（含数组对象），但排除字符串标量。"""
    if obj is None or isinstance(obj, str):
        return False
    return hasattr(obj, 'tolist') or isinstance(obj, (list, tuple))


def _rgba_to_hex(row):
    """将 (r, g, b[, a]) (0-1 浮点) 转为 '#rrggbb' 十六进制颜色。"""
    row = list(row)

    def _ch(v):
        return max(0, min(255, int(round(float(v) * 255))))

    return '#{:02x}{:02x}{:02x}'.format(_ch(row[0]), _ch(row[1]), _ch(row[2]))


def _resolve_scatter_colors(c_vals, cmap, vmin, vmax):
    """把 c 序列解析为传给 scatter_multi 的逐点颜色参数 + colorbar 所需的 mappable 元数据。

    返回 (c_arg, cmap_name, mappable):
    - 颜色字符串序列 / RGB(A) 二维行数组: c_arg 为颜色字符串列表, cmap_name 与 mappable 为 None
    - 数值序列: c_arg 为原始数值列表, cmap_name 为 colormap 名, mappable = (名, lo, hi);
      逐点 RGB 由 Rust 层直接经 colormap_color 求得, 不再经百万级 hex 字符串往返。
    """
    if len(c_vals) == 0:
        return None, None, None
    first = c_vals[0]
    if _is_scatter_sequence(first):  # RGB(A) 二维行数组
        return [_rgba_to_hex(row) for row in c_vals], None, None
    if isinstance(first, str):       # 颜色字符串序列
        return [str(v) for v in c_vals], None, None
    vals = [float(v) for v in c_vals]  # 数值 -> 交由 Rust 直接 colormap 上色
    name = cmap if isinstance(cmap, str) else 'viridis'
    lo = min(vals) if vmin is None else float(vmin)
    hi = max(vals) if vmax is None else float(vmax)
    return vals, name, (name, lo, hi)


def _normalize_scatter(x, y, s, c, marker, label, alpha, edgecolor, linewidth, kwargs):
    """将 matplotlib 风格的 scatter 参数规整为对 Rust 层的调用参数。

    返回 (use_multi, args, mappable):
    - use_multi=False: args = (x, y, s:float, c:str|None, marker, label, alpha, edgecolor, linewidth)
    - use_multi=True:  args = (x, y, s:list|None, c:list|None, marker, label, alpha, edgecolor,
      linewidth, cmap:str|None, vmin:float|None, vmax:float|None)。c 为数值数组时保持原始数值，
      配合 cmap/vmin/vmax 由 Rust 层直接经 colormap_color 上色（避免 hex 字符串往返）。
    - mappable: None 或 (cmap名, vmin, vmax)，当 c 为数值数组经 colormap 映射时给出，
      供随后的 plt.colorbar() 绘制颜色条。
    """
    alpha = alpha if alpha is not None else 1.0
    x = _to_seq(x)
    y = _to_seq(y)
    if hasattr(x, 'ndim') and x.ndim == 0:
        x = [float(x.item()) if hasattr(x, 'item') else float(x)]
    elif not hasattr(x, '__len__'):
        x = [x]
    if hasattr(y, 'ndim') and y.ndim == 0:
        y = [float(y.item()) if hasattr(y, 'item') else float(y)]
    elif not hasattr(y, '__len__'):
        y = [y]
    n = len(x) if hasattr(x, '__len__') else 0
    marker = marker or 'o'
    if c is None:
        c = kwargs.pop('color', None)
    cmap = kwargs.pop('cmap', None)
    vmin = kwargs.pop('vmin', None)
    vmax = kwargs.pop('vmax', None)
    # norm / colorizer / plotnonfinite / data 等参数当前接受但不生效

    s_is_seq = _is_scatter_sequence(s)
    c_is_seq = _is_scatter_sequence(c)

    c_list = None
    c_single = None
    mappable = None
    cmap_name = None
    if c_is_seq:
        # 快路径：一维数值缓冲 c（且非「长度 3/4 单 RGB」歧义）明确是 colormap 数值，
        # 零拷贝直传 Rust，跳过 .tolist() / 逐元素类别检查 / [float(v)] 物化（scatter 热路径）。
        clen = _numeric_buffer_1d_len(c)
        can_fast = clen is not None and not (clen in (3, 4) and clen != n)
        fast_done = False
        if can_fast:
            # 数组库的 .min()/.max() 返回类型可能不稳定（float 或 0 维 ndarray），
            # 用 _reduce_to_float 归一；缺少 min/max 等异常时回退慢路径确保不崩。
            try:
                lo = _reduce_to_float(c.min()) if vmin is None else float(vmin)
                hi = _reduce_to_float(c.max()) if vmax is None else float(vmax)
                cmap_name = cmap if isinstance(cmap, str) else 'viridis'
                c_list = c
                mappable = (cmap_name, lo, hi)
                fast_done = True
            except (TypeError, ValueError, AttributeError):
                cmap_name = None
                c_list = None
                mappable = None
        if not fast_done:
            c_vals = _to_list(c)
            # 单个 RGB(A)（长度 3/4 的纯数值序列，且与点数不同）视为统一单色
            is_flat_numeric = all(
                not isinstance(v, str) and not _is_scatter_sequence(v) for v in c_vals
            )
            if is_flat_numeric and len(c_vals) in (3, 4) and len(c_vals) != n:
                c_single = _rgba_to_hex(c_vals)
            else:
                c_list, cmap_name, mappable = _resolve_scatter_colors(c_vals, cmap, vmin, vmax)
    elif isinstance(c, str):
        c_single = c

    use_multi = s_is_seq or (c_list is not None)
    if not use_multi:
        s_val = 100.0 if s is None else float(s)
        return (False,
                (x, y, s_val, c_single, marker, label, alpha, edgecolor, linewidth),
                mappable)

    if s_is_seq:
        s_arg = [float(v) for v in _to_list(s)]
    elif s is None:
        s_arg = None
    else:
        s_arg = [float(s)] * n

    if c_list is not None:
        c_arg = c_list
    elif c_single is not None:
        c_arg = [c_single] * n
    else:
        c_arg = None

    # 数值 colormap 路径：把 cmap 名与已解析的 lo/hi 传给 Rust，由其直接算 RGB（跳过 hex 往返）。
    # 字符串 / RGB(A) 颜色路径 cmap_name 为 None，Rust 按颜色字符串解析。
    if cmap_name is not None and mappable is not None:
        cmap_arg, vmin_arg, vmax_arg = mappable
    else:
        cmap_arg, vmin_arg, vmax_arg = None, None, None
    return (True,
            (x, y, s_arg, c_arg, marker, label, alpha, edgecolor, linewidth,
             cmap_arg, vmin_arg, vmax_arg),
            mappable)


def _coerce_edgecolor(edgecolors):
    """把 matplotlib 的 edgecolors 归一化为单个颜色字符串 (后端仅支持统一描边色)。

    - 标量颜色字符串 (如 'black' / 'none' / 'face') 原样返回；
    - 单个 RGB(A) 数值序列转为 '#rrggbb'；
    - 逐点颜色序列取首个元素作为整体描边色；
    - 其他情况返回 None (不描边)。
    """
    if edgecolors is None:
        return None
    if isinstance(edgecolors, str):
        return edgecolors
    seq = _to_list(edgecolors)
    if isinstance(seq, (list, tuple)) and len(seq) > 0:
        if len(seq) in (3, 4) and all(
                not isinstance(v, str) and not _is_scatter_sequence(v) for v in seq):
            return _rgba_to_hex(seq)
        first = seq[0]
        if isinstance(first, str):
            return first
        if _is_scatter_sequence(first):
            return _rgba_to_hex(first)
    return None


def _coerce_linewidth(linewidths):
    """把 matplotlib 的 linewidths 归一化为单个浮点数 (后端仅支持统一线宽)。

    标量数值原样转 float；序列取首个元素；无法解析时返回 None (用默认 1.5)。
    """
    if linewidths is None:
        return None
    if isinstance(linewidths, bool):
        return None
    if isinstance(linewidths, (int, float)):
        return float(linewidths)
    seq = _to_list(linewidths)
    if isinstance(seq, (list, tuple)) and len(seq) > 0:
        try:
            return float(seq[0])
        except (TypeError, ValueError):
            return None
    return None


def _get_axes():
    """获取当前 axes，如果没有则返回 None"""
    try:
        return _rsplotlib.gca()
    except Exception:
        return None


def _get_figure():
    """获取当前 figure，如果没有则返回 None"""
    try:
        return _rsplotlib.gcf()
    except Exception:
        return None


def _apply_axes_label(ax, label):
    """将 label 应用到子图（若后端支持 set_label）；否则静默忽略。"""
    if label is not None:
        setter = getattr(ax, 'set_label', None)
        if setter is not None:
            setter(label)


def _route_to_ax(ax_method_name, module_method, *args, **kwargs):
    """将调用路由到当前 axes（如果存在）或模块级函数

    Args:
        ax_method_name: axes 的方法名（字符串）
        module_method: 模块级函数（可调用对象）
        *args: 传递给方法的参数
        **kwargs: 关键字参数（必须同时转发到 axes 端，否则会丢参）
    """
    ax = _get_axes()
    if ax is not None and hasattr(ax, ax_method_name):
        method = getattr(ax, ax_method_name)
        method(*args, **kwargs)
        return _get_figure()
    return module_method(*args, **kwargs)


def _map_aliases(kwargs):
    """规范化 matplotlib 别名到标准名"""
    alias_map = {
        'lw': 'linewidth',
        'c': 'color',
        'ls': 'linestyle',
        'ms': 'markersize',
        'mfc': 'markerfacecolor',
        'mec': 'markeredgecolor',
        'mew': 'markeredgewidth',
    }
    for alias, target in alias_map.items():
        if alias in kwargs and target not in kwargs:
            kwargs[target] = kwargs.pop(alias)
        elif alias in kwargs:
            kwargs.pop(alias)
    # 规范化 linestyle 词形 ('solid'/'dotted'/'dashed'/'dashdot') 与空值到简写，
    # 与 matplotlib 一致：既可写 linestyle='dotted' 也可写 linestyle=':'。
    # 元组形式 (offset, onoffseq)（参数化 dash 图案）编码为 "dashes=..." 串下沉到 Rust。
    ls = kwargs.get('linestyle')
    if isinstance(ls, str):
        key = ls.strip().lower()
        if key == '' or key == 'none':
            kwargs['linestyle'] = ' '  # 空串 / ' ' / 'None' 均表示不画线
        elif key in _LINESTYLE_ALIASES:
            kwargs['linestyle'] = _LINESTYLE_ALIASES[key]
        # 已是简写 ('-' / '--' / ':' / '-.') 时保持不变
    elif isinstance(ls, (tuple, list)):
        enc = _encode_dash_linestyle(ls)
        if enc is not None:
            kwargs['linestyle'] = enc


def _encode_dash_linestyle(ls):
    """matplotlib 元组 linestyle ``(offset, onoffseq)`` -> ``"dashes=<offset>;<v0>,<v1>,..."`` 编码串。

    - 空 / None 的 onoffseq 视为实线 ('-')。
    - 结构非法时返回 None（交由下游忽略, 保持原值）。
    """
    if len(ls) != 2:
        return None
    offset, seq = ls
    if seq is None:
        return '-'
    try:
        seq = list(seq)
    except TypeError:
        return None
    if len(seq) == 0:
        return '-'
    if offset is None:
        offset = 0
    try:
        return "dashes=%g;%s" % (float(offset), ",".join("%g" % float(v) for v in seq))
    except (TypeError, ValueError):
        return None


# linestyle 词形 -> 简写。空串 / 'None' 在 _map_aliases 中单独处理为 ' '(不画线)。
_LINESTYLE_ALIASES = {
    'solid': '-',
    'dotted': ':',
    'dashed': '--',
    'dashdot': '-.',
}


# ==================== 轻量 mathtext (LaTeX -> Unicode) ====================

# LaTeX 命令 -> Unicode。覆盖希腊字母（大小写）与常见数学符号，满足 matplotlib
# mathtext 最常见的用法（如 r'$\mu=100,\ \sigma=15$' 渲染为 'μ=100, σ=15'）。
# 完整的 LaTeX 数学排版（真正的上下标定位、分式、根号盒子）不在支持范围内。
_MATHTEXT_SYMBOLS = {
    'alpha': 'α', 'beta': 'β', 'gamma': 'γ', 'delta': 'δ', 'epsilon': 'ε',
    'varepsilon': 'ε', 'zeta': 'ζ', 'eta': 'η', 'theta': 'θ', 'vartheta': 'ϑ',
    'iota': 'ι', 'kappa': 'κ', 'lambda': 'λ', 'mu': 'μ', 'nu': 'ν', 'xi': 'ξ',
    'omicron': 'ο', 'pi': 'π', 'varpi': 'ϖ', 'rho': 'ρ', 'varrho': 'ϱ',
    'sigma': 'σ', 'varsigma': 'ς', 'tau': 'τ', 'upsilon': 'υ', 'phi': 'φ',
    'varphi': 'φ', 'chi': 'χ', 'psi': 'ψ', 'omega': 'ω',
    'Gamma': 'Γ', 'Delta': 'Δ', 'Theta': 'Θ', 'Lambda': 'Λ', 'Xi': 'Ξ',
    'Pi': 'Π', 'Sigma': 'Σ', 'Upsilon': 'Υ', 'Phi': 'Φ', 'Psi': 'Ψ',
    'Omega': 'Ω',
    'times': '×', 'div': '÷', 'pm': '±', 'mp': '∓', 'cdot': '·',
    'ast': '∗', 'star': '⋆', 'circ': '∘', 'bullet': '•',
    'infty': '∞', 'partial': '∂', 'nabla': '∇', 'forall': '∀', 'exists': '∃',
    'leq': '≤', 'le': '≤', 'geq': '≥', 'ge': '≥', 'neq': '≠', 'ne': '≠',
    'approx': '≈', 'equiv': '≡', 'sim': '∼', 'propto': '∝', 'll': '≪',
    'gg': '≫', 'in': '∈', 'notin': '∉', 'subset': '⊂', 'supset': '⊃',
    'cup': '∪', 'cap': '∩', 'sum': '∑', 'prod': '∏', 'int': '∫',
    'sqrt': '√', 'angle': '∠', 'degree': '°', 'prime': '′', 'ell': 'ℓ',
    'hbar': 'ℏ', 'Re': 'ℜ', 'Im': 'ℑ', 'aleph': 'ℵ', 'emptyset': '∅',
    'rightarrow': '→', 'to': '→', 'leftarrow': '←', 'gets': '←',
    'Rightarrow': '⇒', 'Leftarrow': '⇐', 'leftrightarrow': '↔',
    'Leftrightarrow': '⇔', 'uparrow': '↑', 'downarrow': '↓',
    'langle': '⟨', 'rangle': '⟩', 'cdots': '⋯', 'ldots': '…', 'dots': '…',
    'imath': 'ı', 'jmath': 'ȷ', 'wp': '℘', 'surd': '√', 'neg': '¬',
    'perp': '⊥', 'parallel': '∥', 'therefore': '∴', 'because': '∵',
    'oplus': '⊕', 'otimes': '⊗', 'wedge': '∧', 'vee': '∨', 'setminus': '∖',
}

# 变音符号命令 -> 组合用 Unicode 变音符（跟在基字符之后即叠加其上）。
_ACCENTS = {
    'hat': '\u0302', 'widehat': '\u0302', 'check': '\u030c',
    'tilde': '\u0303', 'widetilde': '\u0303', 'acute': '\u0301',
    'grave': '\u0300', 'bar': '\u0304', 'overline': '\u0305',
    'breve': '\u0306', 'dot': '\u0307', 'ddot': '\u0308',
    'dddot': '\u20db', 'ddddot': '\u20dc', 'vec': '\u20d7',
    'overrightarrow': '\u20d7', 'mathring': '\u030a',
}
# 覆盖每个字符（而非仅首字符）的变音符命令。
_ACCENTS_SPREAD = {'overline', 'widehat', 'widetilde', 'overrightarrow'}

# 字体命令：仅剥离命令本身，递归转换其花括号内的内容。
_FONT_COMMANDS = {
    'mathrm', 'mathit', 'mathtt', 'mathcal', 'mathbb', 'mathfrak',
    'mathsf', 'mathbf', 'mathbfit', 'mathdefault', 'mathregular',
    'mathnormal', 'boldsymbol', 'text', 'textrm', 'textit', 'textbf',
    'operatorname',
}
# \text 系列在数学模式里保留字面空格；其余字体命令与普通数学模式一致（忽略空格）。
_TEXT_COMMANDS = {'text', 'textrm', 'textit', 'textbf'}

# 数学字体命令 -> 样式键。映射到 Unicode 数学字母符号（Mathematical Alphanumeric
# Symbols）。未列出的字体命令（mathrm/mathnormal/text/operatorname 等）视为默认
# 罗马体，仅剥离命令、内容不改字形。
_MATH_FONT_STYLES = {
    'mathbf': 'bf', 'boldsymbol': 'bf', 'textbf': 'bf',
    'mathit': 'it', 'textit': 'it',
    'mathbfit': 'bfit',
    'mathcal': 'cal',
    'mathfrak': 'frak',
    'mathbb': 'bb',
    'mathsf': 'sf',
    'mathtt': 'tt',
}
# 每种样式：(大写字母基址, 小写字母基址, 数字基址或 None, 例外洞表)。
# 例外洞：部分字形在 SMP 数学字母块中留空，Unicode 把它们放到 BMP 的
# Letterlike Symbols 区（如 \mathcal{R}=ℛ U+211B、\mathbb{R}=ℝ U+211D）。
_MATH_ALPHA = {
    'bf': (0x1D400, 0x1D41A, 0x1D7CE, {}),
    'it': (0x1D434, 0x1D44E, None, {'h': 0x210E}),
    'bfit': (0x1D468, 0x1D482, None, {}),
    'cal': (0x1D49C, 0x1D4B6, None, {
        'B': 0x212C, 'E': 0x2130, 'F': 0x2131, 'H': 0x210B, 'I': 0x2110,
        'L': 0x2112, 'M': 0x2133, 'R': 0x211B, 'e': 0x212F, 'g': 0x210A,
        'o': 0x2134}),
    'frak': (0x1D504, 0x1D51E, None, {
        'C': 0x212D, 'H': 0x210C, 'I': 0x2111, 'R': 0x211C, 'Z': 0x2128}),
    'bb': (0x1D538, 0x1D552, 0x1D7D8, {
        'C': 0x2102, 'H': 0x210D, 'N': 0x2115, 'P': 0x2119, 'Q': 0x211A,
        'R': 0x211D, 'Z': 0x2124}),
    'sf': (0x1D5A0, 0x1D5BA, 0x1D7E2, {}),
    'tt': (0x1D670, 0x1D68A, 0x1D7F6, {}),
}


def _style_char(ch, style):
    """把单个 ASCII 字母/数字映射为对应数学字体的 Unicode 字符。

    非字母/数字、或该样式无对应字形（如斜体数字）时原样返回。若目标字形不被
    实际渲染字体支持（如 macOS Arial Unicode 缺 SMP 数学字母块），回退为原字符，
    避免渲染出缺字形方框。
    """
    if not style:
        return ch
    spec = _MATH_ALPHA.get(style)
    if spec is None:
        return ch
    up, low, dig, holes = spec
    if ch in holes:
        cp = holes[ch]
    elif 'A' <= ch <= 'Z':
        cp = up + (ord(ch) - ord('A'))
    elif 'a' <= ch <= 'z':
        cp = low + (ord(ch) - ord('a'))
    elif dig is not None and '0' <= ch <= '9':
        cp = dig + (ord(ch) - ord('0'))
    else:
        return ch
    mapped = chr(cp)
    try:
        if not _rsplotlib.glyph_supported(mapped):
            return ch
    except Exception:
        pass
    return mapped


# 罗马体函数名：原样输出名称本身（如 \sin -> "sin"）。
_FUNCTION_NAMES = {
    'sin', 'cos', 'tan', 'cot', 'sec', 'csc', 'sinh', 'cosh', 'tanh',
    'coth', 'arcsin', 'arccos', 'arctan', 'exp', 'log', 'ln', 'lg',
    'det', 'dim', 'ker', 'deg', 'gcd', 'hom', 'lim', 'liminf', 'limsup',
    'max', 'min', 'sup', 'inf', 'arg', 'Pr', 'sgn',
}
# 需要吞掉一个 {..} 尺寸参数、输出等宽空白的间距 / 占位命令。
_SPACE_GROUP_COMMANDS = {
    'hspace', 'mspace', 'kern', 'mkern', 'phantom', 'hphantom', 'vphantom',
}
# 无参数间距命令 -> 空格。
_SPACE_COMMANDS = {
    'quad', 'qquad', 'thinspace', 'medspace', 'thickspace', 'space',
    'enspace', 'negthinspace',
}
# 反斜杠 + 单个非字母字符 -> 空格（TeX 显式间距）。
_BACKSLASH_SPACE = set(' ,;:><./')

# 结构化数学构造的中间表示（IR）控制字符，交给 Rust 二维排版引擎解析。
# 必须与 src/utils/mathtext.rs 中的常量完全一致：
#   script:  START 's' base SEP sup SEP sub END
#   frac:    START 'f' num SEP den END           （带分数线）
#   binom:   START 'b' num SEP den END            （括号，无线）
#   genfrac: START 'g' ld SEP rd SEP bar SEP num SEP den END
#   sqrt:    START 'r' index SEP body END          （index 为空表示平方根）
_IR_START = '\u0002'
_IR_SEP = '\u001f'
_IR_END = '\u0003'


def _read_group(expr, i):
    """expr[i] 为 '{'，返回 (组内内容, 右括号之后的下标)。允许花括号嵌套。"""
    depth, i, buf = 1, i + 1, []
    n = len(expr)
    while i < n and depth > 0:
        c = expr[i]
        if c == '{':
            depth += 1
            buf.append(c)
        elif c == '}':
            depth -= 1
            if depth > 0:
                buf.append(c)
        else:
            buf.append(c)
        i += 1
    return ''.join(buf), i


def _read_command(expr, i):
    """expr[i] 为 '\\'，返回 (命令名或单字符, 之后的下标)。

    命令名是紧随的连续字母；若反斜杠后为非字母，则命令为该单个字符。
    与 TeX 一致：字母命令名后的空白被忽略（这样 '\\hat i' 的重音作用于 i）。
    """
    n = len(expr)
    j = i + 1
    if j < n and expr[j].isalpha():
        k = j
        while k < n and expr[k].isalpha():
            k += 1
        cmd = expr[j:k]
        while k < n and expr[k] == ' ':
            k += 1
        return cmd, k
    if j < n:
        return expr[j], j + 1
    return '', j


def _read_atom(expr, i, keep_spaces, font_style=None):
    """读取上/下标作用的“原子”，返回 (已转换的 Unicode 串, 新下标)。

    原子可为 {..} 组、\\命令、或单个字符。
    """
    n = len(expr)
    if i >= n:
        return '', i
    ch = expr[i]
    if ch == '{':
        content, i = _read_group(expr, i)
        return _convert_math(content, keep_spaces, font_style), i
    if ch == '\\':
        _, after = _read_command(expr, i)
        return _convert_math(expr[i:after], keep_spaces, font_style), after
    return _style_char(ch, font_style), i + 1


def _thickness_is_zero(spec):
    """genfrac 的分数线粗细参数是否表示“无线”（堆叠数字）。

    空串表示默认线宽（有线）；数值 0（含 '0pt'/'0mm' 等单位）表示无线。
    """
    spec = spec.strip()
    if not spec:
        return False
    for unit in ('pt', 'mm', 'cm', 'in', 'em', 'ex', 'px'):
        if spec.endswith(unit):
            spec = spec[:-len(unit)].strip()
            break
    try:
        return float(spec) == 0.0
    except ValueError:
        return False


def _convert_math(expr, keep_spaces=False, font_style=None):
    """把一段数学模式文本（$...$ 内部）转换为可供渲染的字符串。

    希腊字母/符号/字体命令/函数名/变音符号/间距等转换为 Unicode 文本；
    结构化构造（^ / _ 上下标、\\frac/\\binom/\\genfrac、\\sqrt[n]{}）编码为
    控制字符 IR（见 _IR_START 等），交给 Rust 二维排版引擎堆叠、画分数线/
    根号盖线；无法承载二维排版的渲染站点会把 IR 降级为单行 Unicode 近似。
    支持范围：希腊字母与常见符号命令；字体命令
    (\\mathrm/\\mathcal/\\text ... 剥离保留内容)；函数名 (\\sin -> "sin")；
    ^ / _ 上下标；\\frac/\\binom/\\genfrac；\\sqrt[n]{}；变音符号
    (\\hat/\\bar/\\vec ... 组合字符)；间距命令 (\\hspace{}/\\,/\\;/\\quad ...)；
    \\left/\\right（丢弃）。数学模式忽略字面空格（\\text{} 内保留）。
    """
    out = []
    i, n = 0, len(expr)
    while i < n:
        ch = expr[i]
        if ch == '\\':
            cmd, after = _read_command(expr, i)
            if cmd == '':
                i = after
                continue
            if len(cmd) == 1 and not cmd.isalpha():
                if cmd in _BACKSLASH_SPACE:
                    out.append(' ')
                elif cmd == '!':
                    pass                       # \! 负间距 -> 忽略
                elif cmd == '\\':
                    out.append('\n')           # \\ -> 换行
                else:
                    out.append(cmd)            # \$ \% \# \& \_ \{ \} \| 等 -> 字面
                i = after
                continue
            if cmd in _ACCENTS:
                atom, i = _read_atom(expr, after, keep_spaces, font_style)
                comb = _ACCENTS[cmd]
                if cmd in _ACCENTS_SPREAD and atom:
                    out.append(''.join(c + comb for c in atom))
                elif atom:
                    out.append(atom[0] + comb + atom[1:])
                else:
                    out.append(comb)
                continue
            if cmd in _FONT_COMMANDS:
                # 字体命令：进入其花括号内容并切换字体样式。未列入 _MATH_FONT_STYLES
                # 的命令（mathrm/text/… 罗马体）把样式重置为默认（None）。
                new_style = _MATH_FONT_STYLES.get(cmd)
                if after < n and expr[after] == '{':
                    content, i = _read_group(expr, after)
                    out.append(_convert_math(
                        content, keep_spaces or cmd in _TEXT_COMMANDS,
                        new_style))
                else:
                    i = after
                continue
            if cmd == 'sqrt':
                i = after
                root = ''
                if i < n and expr[i] == '[':
                    end = expr.find(']', i)
                    if end != -1:
                        root, i = expr[i + 1:end], end + 1
                if i < n and expr[i] == '{':
                    content, i = _read_group(expr, i)
                    body = _convert_math(content, keep_spaces, font_style)
                else:
                    body = ''
                index = (_convert_math(root, keep_spaces, font_style)
                         if root else '')
                out.append(_IR_START + 'r' + index + _IR_SEP + body + _IR_END)
                continue
            if cmd in ('frac', 'dfrac', 'tfrac', 'binom', 'dbinom', 'tbinom'):
                i = after
                parts = []
                for _ in range(2):
                    if i < n and expr[i] == '{':
                        grp, i = _read_group(expr, i)
                        parts.append(_convert_math(grp, keep_spaces, font_style))
                    else:
                        parts.append('')
                kind = 'b' if cmd.endswith('binom') else 'f'
                out.append(''.join(
                    (_IR_START, kind, parts[0], _IR_SEP, parts[1], _IR_END)))
                continue
            if cmd == 'genfrac':
                i = after
                groups = []
                while len(groups) < 6 and i < n and expr[i] == '{':
                    grp, i = _read_group(expr, i)
                    groups.append(grp)
                groups += [''] * (6 - len(groups))
                ld = _convert_math(groups[0], keep_spaces, font_style)
                rd = _convert_math(groups[1], keep_spaces, font_style)
                bar_flag = '0' if _thickness_is_zero(groups[2]) else '1'
                num = _convert_math(groups[4], keep_spaces, font_style)
                den = _convert_math(groups[5], keep_spaces, font_style)
                out.append(''.join(
                    (_IR_START, 'g', ld, _IR_SEP, rd, _IR_SEP,
                     bar_flag, _IR_SEP, num, _IR_SEP, den, _IR_END)))
                continue
            if cmd in _SPACE_GROUP_COMMANDS:
                i = after
                if i < n and expr[i] == '{':
                    _, i = _read_group(expr, i)   # 丢弃尺寸 / 占位内容
                out.append(' ')
                continue
            if cmd in ('left', 'right'):
                i = after
                if i < n and expr[i] == '.':
                    i += 1                        # \left. / \right. 隐形定界符
                continue
            if cmd in _SPACE_COMMANDS:
                out.append(' ')
                i = after
                continue
            if cmd in _FUNCTION_NAMES:
                out.append(cmd)
                i = after
                continue
            out.append(_MATHTEXT_SYMBOLS.get(cmd, ''))  # 普通符号，未知命令丢弃
            i = after
            continue
        if ch in '^_':
            # 上/下标作用于紧邻的前一个原子（out 的最后一项）；连续的
            # ^ 与 _ 合并到同一 base，交给 Rust 二维排版为真正的上下标。
            base = out.pop() if out else ''
            sup = sub = ''
            atom, i = _read_atom(expr, i + 1, keep_spaces, font_style)
            if ch == '^':
                sup = atom
            else:
                sub = atom
            if i < n and expr[i] in '^_':
                ch2 = expr[i]
                atom2, i = _read_atom(expr, i + 1, keep_spaces, font_style)
                if ch2 == '^':
                    sup = atom2
                else:
                    sub = atom2
            out.append(''.join(
                (_IR_START, 's', base, _IR_SEP, sup, _IR_SEP, sub, _IR_END)))
            continue
        if ch == '{':
            content, i = _read_group(expr, i)
            out.append(_convert_math(content, keep_spaces, font_style))
            continue
        if ch == '}':
            i += 1
            continue
        if ch == '~':
            out.append(' ')                       # ~ 不折行空格
            i += 1
            continue
        if ch == ' ' and not keep_spaces:
            i += 1                                # 数学模式忽略字面空格
            continue
        out.append(_style_char(ch, font_style))
        i += 1
    return ''.join(out)


def _split_dollar(s):
    """按未转义的 '$' 把字符串切成 (is_math, text) 段序列。

    '\\$' 视为字面 '$' 并入所在段。返回 (segments, balanced)；balanced 为
    False 表示未转义 '$' 为奇数个（matplotlib 语义：整串按普通文本处理）。
    """
    segments, buf = [], []
    is_math, count = False, 0
    i, n = 0, len(s)
    while i < n:
        c = s[i]
        if c == '\\' and i + 1 < n and s[i + 1] == '$':
            buf.append('$')                       # 转义的字面美元符
            i += 2
            continue
        if c == '$':
            segments.append((is_math, ''.join(buf)))
            buf, is_math, count = [], not is_math, count + 1
            i += 1
            continue
        buf.append(c)
        i += 1
    segments.append((is_math, ''.join(buf)))
    return segments, count % 2 == 0


def _render_mathtext(s):
    """把 matplotlib 风格的 mathtext ($...$) 转换为 Unicode 文本。

    成对 $...$ 之间按数学模式转换，之外保持字面；'\\$' 为字面美元符。
    未转义 '$' 为奇数个时整串按普通文本处理（仅把 '\\$' 归一为 '$'）。
    非字符串或不含 '$' 时快速返回原值。
    """
    if not isinstance(s, str) or '$' not in s:
        return s
    segments, balanced = _split_dollar(s)
    if not balanced:
        return s.replace('\\$', '$')
    return ''.join(
        _convert_math(seg) if is_math else seg for is_math, seg in segments)


def _parse_fmt(fmt):
    """解析 matplotlib 风格的格式字符串 '[marker][line][color]'。

    fmt 由三部分任意组合而成 (均可省略):
        marker: 数据点标记, 如 'o' 圆, '^' 三角, 's' 方块, '*' 星 ...
        line:   线型, '-' 实线, '--' 虚线, '-.' 点划线, ':' 点线
        color:  颜色单字母代码 b/g/r/c/m/y/k/w

    例如 'o:r' = 圆形标记 + 点线 + 红色。

    注意 black 与 blue 首字母冲突: 按 matplotlib 约定,
    单字母代码里 'b' 表示 blue, 'k' 才表示 black, 因此不会产生歧义。

    返回 dict, 可能包含 'marker' / 'linestyle' / 'color' 键。
    """
    if not fmt:
        return {}
    line_styles_multi = ('--', '-.')
    line_styles_single = ('-', ':')
    markers = set(".,ov^<>12348spP*hH+xXDd|_")
    color_map = {
        'b': 'blue', 'g': 'green', 'r': 'red', 'c': 'cyan',
        'm': 'magenta', 'y': 'yellow', 'k': 'black', 'w': 'white',
    }

    marker = linestyle = color = None
    i, n = 0, len(fmt)
    while i < n:
        two = fmt[i:i + 2]
        ch = fmt[i]
        if two in line_styles_multi:
            if linestyle is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的线型")
            linestyle, i = two, i + 2
        elif ch in line_styles_single:
            if linestyle is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的线型")
            linestyle, i = ch, i + 1
        elif ch in markers:
            if marker is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的标记")
            marker, i = ch, i + 1
        elif ch == 'C' and i + 1 < n and fmt[i + 1].isdigit():
            # matplotlib 'CN' 颜色循环记号 ('C0'..'C9')：C 后跟数字，交由后端解析。
            if color is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的颜色")
            j = i + 1
            while j < n and fmt[j].isdigit():
                j += 1
            color, i = fmt[i:j], j
        elif ch in color_map:
            if color is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的颜色")
            color, i = color_map[ch], i + 1
        else:
            raise ValueError(f"无法识别的格式字符串 {fmt!r} (非法字符 {ch!r})")

    result = {}
    if marker is not None:
        result['marker'] = marker
    if linestyle is not None:
        result['linestyle'] = linestyle
    elif marker is not None:
        # fmt 指定了 marker 但未指定线型：与 matplotlib 一致，只画标记不画线
        # （' ' 为本库的"无线"表示，见 _map_aliases）。
        result['linestyle'] = ' '
    if color is not None:
        result['color'] = color
    return result


def _implicit_x(y):
    """plot(y) 的隐式横坐标 = [0, 1, ..., len(y)-1]。"""
    try:
        n = len(y)
    except Exception:
        return list(y) if hasattr(y, '__iter__') else []
    return list(range(n))


def _parse_plot_args(args, kwargs):
    """解析 plot() 的位置参数为 [(x, y, fmt_or_none), ...] 对列表 + kwargs

    支持的调用形式 (与 matplotlib 一致):
        plot(y)                    plot(x, y)
        plot(y, fmt)               plot(x, y, fmt)
        plot(x1, y1, fmt1, x2, y2, fmt2, ...)   # 多条线, 每组 fmt 可选
    """
    args = list(args)
    pairs = []
    while args:
        # 取出本组数据参数 (1~2 个); 若第 2 个其实是 fmt 字符串, 则本组只有 1 个
        group = args[:2]
        if len(group) == 2 and isinstance(group[1], str):
            group = group[:1]
        args = args[len(group):]
        # 消费紧跟其后的 fmt 字符串 (若有)
        fmt = None
        if args and isinstance(args[0], str):
            fmt, args = args[0], args[1:]

        if len(group) == 1:
            y = group[0]
            x = _implicit_x(y)
            pairs.append((x, y, fmt))
        else:
            pairs.append((group[0], group[1], fmt))
    return pairs, kwargs


# ==================== 绘图函数 ====================

def plot(*args, **kwargs):
    """绘制折线图。

    用法:
        plt.plot(x, y)              # 以 x 为横坐标, y 为纵坐标
        plt.plot(y)                 # 仅提供 y, 自动 x = [0, 1, ...]
        plt.plot(x, y, lw=2.0)     # 自定义线宽
        plt.plot(x, y, x, z)       # 绘制多条线
        plt.plot(y, 'o:r')         # 格式字符串: 圆标记 + 点线 + 红色

    格式字符串 fmt = '[marker][line][color]', 各部分均可省略, 例如:
        'o'   仅圆形标记        '--'  仅虚线
        'o:r' 圆标记+点线+红色  '^-g' 三角标记+实线+绿色
    颜色单字母: b=blue g=green r=red c=cyan m=magenta y=yellow k=black w=white
    (注意 black 用 'k' 而非 'b', 'b' 表示 blue, 避免与 blue 首字母冲突)
    显式关键字参数优先于 fmt 中的同名设置。

    关键字参数 (matplotlib 兼容别名):
        lw / linewidth: 线宽 (float)
        c / color: 颜色 (如 'red', '#FF0000')
        ls / linestyle: 线型 ('-', '--', ':', '-.')
        marker: 数据点标记 ('o', 's', '^', 'D', '*', 'x', '+')
        ms / markersize: 标记大小 (float)
        mfc / markerfacecolor: 标记内部填充色
        mec / markeredgecolor: 标记边框色
        solid_capstyle: 端点 ('butt', 'round', 'projecting')
        label: 图例标签

    Returns:
        (Figure, Axes) 元组
    """
    _map_aliases(kwargs)
    pairs, _ = _parse_plot_args(args, kwargs)

    def _call(*a, **k):
        return _rsplotlib.plot(*a, **k)

    result = None
    for x, y, fmt in pairs:
        call_kwargs = dict(kwargs)
        if fmt:
            # fmt 解析出的样式作为默认值, 不覆盖用户显式传入的关键字参数
            for key, value in _parse_fmt(fmt).items():
                call_kwargs.setdefault(key, value)
        # 分类坐标：字符串 x/y 映射到 0,1,2,... 位置，字符串作为刻度标签。
        x, x_tick_labels = _categorical(x)
        y, y_tick_labels = _categorical(y)
        # 将数组对象转换为 Python list，避免类型不一致问题
        x = _to_list(x)
        y = _to_list(y)
        result = _route_to_ax('plot', _call, x, y, **call_kwargs)
        if x_tick_labels is not None:
            xticks(list(range(len(x_tick_labels))), x_tick_labels)
        if y_tick_labels is not None:
            yticks(list(range(len(y_tick_labels))), y_tick_labels)
    return result


def scatter(x, y, s=None, c=None, marker=None, cmap=None, norm=None,
            vmin=None, vmax=None, alpha=None, linewidths=None,
            edgecolors=None, colorizer=None, plotnonfinite=False,
            data=None, **kwargs):
    """绘制散点图，兼容 matplotlib.pyplot.scatter 的参数签名。

    用法:
        plt.scatter(x, y)                              # 默认大小 100、默认蓝色
        plt.scatter(x, y, s=50, c='red')               # 统一大小和颜色
        plt.scatter(x, y, s=[10, 20, 30], c=['r','g','b'])   # 逐点大小/颜色
        plt.scatter(x, y, c=values, cmap='viridis')    # 数值经 colormap 映射
        plt.scatter(x, y, c=[[1,0,0],[0,1,0]])         # RGB(A) 二维行数组
        plt.scatter(x, y, edgecolors='black', linewidths=1.5)  # 黑色描边

    Args:
        x, y: 长度相同的数据点坐标 (list / tuple / array)
        s: 点大小, 默认 100; 可为标量或与点数等长的数组
        c: 颜色; 默认蓝色; 可为颜色字符串、颜色字符串数组、数值数组
           (配合 cmap) 或 RGB(A) 二维行数组
        marker: 标记形状, 默认 'o'
        cmap: 当 c 为数值数组时使用的 colormap 名称 (如 'viridis')
        vmin, vmax: colormap 归一化范围
        alpha: 透明度 (0.0 - 1.0)
        linewidths: 标记边缘线宽 (points); 后端取统一线宽, 序列取首个元素
        edgecolors: 标记边缘颜色; 'face'/'none' 表示不额外描边, 其他颜色启用描边
            (后端仅支持统一描边色, 逐点颜色取首个元素)。也接受单数别名 edgecolor。
        norm / colorizer / plotnonfinite: 接受但当前不生效
        data: 若提供 (如 dict / DataFrame)，x/y/s/c 等字符串参数将按键在 data 中查找取值
        **kwargs: 额外关键字参数 (color 将作为 c 的别名)
    """
    if data is not None:
        # matplotlib 兼容：以字符串键索引 data 得到实际数据。
        x = _replace_from_data(x, data)
        y = _replace_from_data(y, data)
        s = _replace_from_data(s, data)
        c = _replace_from_data(c, data)
        if 'color' in kwargs:
            kwargs['color'] = _replace_from_data(kwargs['color'], data)
    # 单数形式 edgecolor / linewidth 作为别名 (matplotlib 主用复数名)。
    if edgecolors is None:
        edgecolors = kwargs.pop('edgecolor', None)
    else:
        kwargs.pop('edgecolor', None)
    if linewidths is None:
        linewidths = kwargs.pop('linewidth', None)
    else:
        kwargs.pop('linewidth', None)
    edgecolor = _coerce_edgecolor(edgecolors)
    linewidth = _coerce_linewidth(linewidths)
    kwargs['cmap'] = cmap
    kwargs['vmin'] = vmin
    kwargs['vmax'] = vmax
    a = 1.0 if alpha is None else alpha
    label = kwargs.pop('label', None)
    # 分类坐标：字符串 x/y 映射到 0,1,2,... 位置，字符串作为刻度标签。
    x, x_tick_labels = _categorical(x)
    y, y_tick_labels = _categorical(y)
    use_multi, args, mappable = _normalize_scatter(
        x, y, s, c, marker, label=label, alpha=a,
        edgecolor=edgecolor, linewidth=linewidth, kwargs=kwargs)
    if use_multi:
        result = _route_to_ax('scatter_multi', _rsplotlib.scatter_multi, *args)
    else:
        result = _route_to_ax('scatter', _rsplotlib.scatter, *args)
    if x_tick_labels is not None:
        xticks(list(range(len(x_tick_labels))), x_tick_labels)
    if y_tick_labels is not None:
        yticks(list(range(len(y_tick_labels))), y_tick_labels)
    if mappable is not None:
        ax = _get_axes()
        if ax is not None and hasattr(ax, 'set_mappable'):
            ax.set_mappable(*mappable)
    return result


def bar(x, height, width=0.8, color=None, label=None):
    """绘制柱状图。

    Args:
        x: 每个柱子的 x 坐标 (list / tuple / array)
        height: 每个柱子的高度 (y 值)
        width: 柱子的宽度 (默认 0.8)
        color: 柱子的颜色字符串
        label: 图例标签

    用法:
        plt.bar([0, 1, 2], [1, 2, 3])
        plt.bar(["A", "B", "C"], [1, 2, 3])  # 字符串 x 作为类别标签
    """
    x = _to_list(x)
    height = _to_seq(height)
    # 类别型 x：x 为字符串序列时，柱子落在 0,1,2,... 位置，字符串作为 x 轴刻度标签。
    tick_labels = None
    if isinstance(x, (list, tuple)) and any(isinstance(v, str) for v in x):
        tick_labels = [str(v) for v in x]
        x = list(range(len(x)))
    result = _route_to_ax('bar', _rsplotlib.bar, x, height, width, color, label)
    if tick_labels is not None:
        xticks(x, tick_labels)
    return result


def barh(y, width, height=0.8, color=None, label=None):
    """绘制水平柱状图。

    Args:
        y: 每个柱子的 y 坐标
        width: 每个柱子的宽度 (x 方向长度)
        height: 柱子的高度 (y 方向, 默认 0.8)
        color: 颜色
        label: 图例标签
    """
    y = _to_list(y)
    width = _to_seq(width)
    # 类别型 y：y 为字符串序列时，柱子落在 0,1,2,... 位置，字符串作为 y 轴刻度标签。
    tick_labels = None
    if isinstance(y, (list, tuple)) and any(isinstance(v, str) for v in y):
        tick_labels = [str(v) for v in y]
        y = list(range(len(y)))
    result = _route_to_ax('barh', _rsplotlib.barh, y, width, height, color, label)
    if tick_labels is not None:
        yticks(y, tick_labels)
    return result


def hist(x, bins=None, range=None, density=False, weights=None,
         cumulative=False, bottom=None, histtype='bar', align='mid',
         orientation='vertical', rwidth=None, log=False, color=None,
         label=None, stacked=False, **kwargs):
    """绘制直方图。

    用法:
        plt.hist(data, bins=20)
        plt.hist([data1, data2], bins=10, color=['red', 'blue'], stacked=True)
        plt.hist(data, histtype='step', cumulative=True, density=True)
        plt.hist(data, orientation='horizontal', log=True)

    Args:
        x: 数据 (一维数组, 或多组数据组成的列表)
        bins: 分箱数量 (默认 10) 或箱边界列表
        range: 值域范围 (lo, hi)，None 表示使用数据的最小/最大值
        density: 是否归一化为概率密度 (默认 False)
        weights: 每个数据点的权重
        cumulative: 是否绘制累积分布 (True/False/-1)
        bottom: 每个柱子的起始基线 (默认 0)
        histtype: 'bar' | 'barstacked' | 'step' | 'stepfilled'
        align: 'left' | 'mid' | 'right'
        orientation: 'vertical' | 'horizontal'
        rwidth: 每个柱子相对于分箱宽度的比例 (0~1)
        log: 计数轴是否使用对数刻度
        color: 颜色或颜色列表
        label: 图例标签
        stacked: 是否堆叠多组直方图
        **kwargs: 额外关键字参数 (facecolor, alpha)
    """
    facecolor = kwargs.pop('facecolor', None)
    alpha = kwargs.pop('alpha', 1.0)

    if bins is None:
        bins = 10

    # 数据规整为“组的列表”。一维纯数值缓冲直接下沉给 Rust 零拷贝读取，避免
    # _to_list_recursive + [list(x)] 把百万级数据点物化成 Python 对象（hist 大数据热路径）。
    if _numeric_buffer_1d_len(x) is not None:
        x_list = x
        n_datasets = 1
    else:
        x = _to_list_recursive(x)
        if x and isinstance(x[0], (list, tuple)):
            x_list = [list(v) for v in x]
        else:
            x_list = [list(x)]
        n_datasets = len(x_list)

    # weights 规整为与 x 平行的结构（一维数值缓冲同样直传，Rust 侧零拷贝读取）
    if weights is not None:
        if _numeric_buffer_1d_len(weights) is not None:
            weights_arg = weights
        else:
            w = _to_list_recursive(weights)
            if w and isinstance(w[0], (list, tuple)):
                weights_arg = [list(v) for v in w]
            else:
                weights_arg = [list(w)]
    else:
        weights_arg = None

    # bins: 整数箱数 或 箱边界列表
    if isinstance(bins, (list, tuple)):
        bins_arg = [float(b) for b in bins]
    elif hasattr(bins, 'tolist'):
        bins_arg = [float(b) for b in bins.tolist()]
    else:
        bins_arg = int(bins)

    def _norm_color(c):
        if c is None:
            return None
        if isinstance(c, str):
            return [c] * n_datasets
        if isinstance(c, (list, tuple)):
            return list(c)
        return None

    color_arg = _norm_color(color)
    facecolor_arg = _norm_color(facecolor)

    # cumulative -> int (True=1, False=0, -1=反向累积)
    if cumulative is True:
        cum = 1
    elif cumulative is False or cumulative is None:
        cum = 0
    else:
        cum = int(cumulative)

    range_arg = tuple(range) if range is not None else None

    hist_kwargs = dict(
        bins=bins_arg, range=range_arg, density=density, weights=weights_arg,
        cumulative=cum, bottom=bottom, histtype=histtype, align=align,
        orientation=orientation, rwidth=rwidth, log=log, color=color_arg,
        facecolor=facecolor_arg, label=label, stacked=stacked, alpha=alpha,
    )

    ax = _get_axes()
    if ax is not None and hasattr(ax, 'hist'):
        return ax.hist(x_list, **hist_kwargs)
    return _rsplotlib.hist(x_list, **hist_kwargs)


def pie(x, labels=None, colors=None, autopct=False, startangle=0.0, explode=None, **kwargs):
    """绘制饼图。

    用法:
        plt.pie([30, 40, 30], labels=['A', 'B', 'C'])

    Args:
        x: 数据列表 (各部分数值)
        labels: 每部分的标签列表
        colors: 每部分的颜色列表
        autopct: 百分比格式字符串 (如 '%1.1f%%'), 或布尔值 True
        startangle: 起始角度 (度), 默认从 x 轴正方向逆时针画起
        explode: 各扇形沿半径方向向外偏移的比例列表 (如 (0, 0.1, 0, 0))
        **kwargs: 其他关键字参数
    """
    x = _to_list(x)
    if autopct and isinstance(autopct, str):
        autopct_str = autopct
    elif autopct:
        autopct_str = "%1.1f%%"
    else:
        autopct_str = None
    explode_list = _to_list(explode) if explode is not None else None
    return _route_to_ax('pie', _rsplotlib.pie, x, labels, colors, autopct_str, startangle, explode_list)


def boxplot(x, labels=None, vert=True, **kwargs):
    """绘制箱线图 (box-and-whisker plot)。

    展示数据的中位数、四分位、离群值等统计信息。

    用法:
        plt.boxplot([data1, data2, data3])

    Args:
        x: 数据集 (可以是一个或多个一维数组)
        labels: 每个箱的标签列表
        vert: 是否垂直绘制 (默认 True)
    """
    # 纯数值缓冲数组直接下沉给 Rust 零拷贝读取（py_to_vec_vec_f64：一维→单箱，
    # 二维→按行拆多箱，与旧 _to_list_recursive 语义一致），避免物化百万级数据点。
    # Python list（含 list of arrays）、含字符串等非缓冲对象保持原路径。
    if not _is_numeric_buffer(x):
        x = _to_list_recursive(x)
    return _route_to_ax('boxplot', _rsplotlib.boxplot, x, labels, vert)


def fill_between(x, y1, y2=0.0, color=None, alpha=0.3, label=None, **kwargs):
    """填充两条曲线之间的区域。

    用法:
        plt.fill_between(x, y1, y2, color='red', alpha=0.3)

    Args:
        x: x 坐标数据
        y1: 第一条曲线的 y 坐标
        y2: 第二条曲线的 y 坐标 (默认 0.0)
        color: 填充颜色
        alpha: 透明度 (0.0-1.0, 默认 0.3)
        label: 图例标签
    """
    x = _to_seq(x)
    y1 = _to_seq(y1)
    y2 = _to_seq(y2)
    return _route_to_ax('fill_between', _rsplotlib.fill_between, x, y1, y2, color, alpha, label)


def errorbar(x, y, yerr=None, xerr=None, fmt='o', color=None, label=None, capsize=3.0, **kwargs):
    """绘制带误差棒的图。

    用法:
        plt.errorbar(x, y, yerr=0.1)

    Args:
        x: x 坐标数据
        y: y 坐标数据
        yerr: y 方向误差 (标量或数组)
        xerr: x 方向误差 (标量或数组)
        fmt: 数据点/线格式字符串 (默认 'o')
        color: 颜色
        label: 图例标签
        capsize: 误差棒末端横线长度 (默认 3.0)
    """
    x = _to_seq(x)
    y = _to_seq(y)
    yerr = _to_seq(yerr)
    xerr = _to_seq(xerr)
    return _route_to_ax('errorbar', _rsplotlib.errorbar, x, y, yerr, xerr, fmt, color, label, capsize)


def stem(x, y, linefmt=None, markerfmt=None, label=None, **kwargs):
    """绘制茎叶图 (火柴杆图)。

    Args:
        x: x 坐标数据
        y: y 坐标数据
        linefmt: 线样式
        markerfmt: 标记样式
        label: 图例标签
    """
    x = _to_seq(x)
    y = _to_seq(y)
    return _route_to_ax('stem', _rsplotlib.stem, x, y, linefmt or '-', markerfmt or 'o', label)


def step(x, y, where='pre', label=None, color=None, linestyle='-', linewidth=1.5, **kwargs):
    """绘制阶梯图。

    Args:
        x: x 坐标数据
        y: y 坐标数据
        where: 阶梯位置 ('pre', 'mid', 'post', 默认 'pre')
        label: 图例标签
        color: 颜色
        linestyle: 线型
        linewidth: 线宽
    """
    x = _to_seq(x)
    y = _to_seq(y)
    return _route_to_ax('step', _rsplotlib.step, x, y, where, label, color, linestyle, linewidth)


def stackplot(x, *args, labels=None, colors=None, alpha=1.0, **kwargs):
    """绘制堆叠面积图。

    用法:
        plt.stackplot(x, y1, y2, y3, labels=['A', 'B', 'C'])

    Args:
        x: x 坐标数据
        *args: 多个 y 数据集
        labels: 每个数据集的标签列表
        colors: 每个数据集的颜色列表
        alpha: 透明度 (默认 1.0)
    """
    x = _to_seq(x)
    y_data = [_to_seq(a) for a in args if a is not None]
    if not y_data and 'y' in kwargs:
        y_data = [_to_seq(kwargs['y'])]
    return _route_to_ax('stackplot', _rsplotlib.stackplot, x, *y_data,
                        labels=labels, colors=colors, alpha=alpha)


def imshow(x, cmap=None, norm=None, aspect=None, interpolation=None,
           alpha=None, vmin=None, vmax=None, origin=None, extent=None, **kwargs):
    """显示图像 (矩阵热力图 / 灰度图 / RGB 彩色图)。

    Args:
        x: 图像数据。2D 数组 (行->y 轴, 列->x 轴) 经 cmap 上色；
           3D 数组 (H, W, 3/4) 视为 RGB(A) 彩色图，直接按像素颜色绘制
           (浮点取值 [0,1]，整数取值 [0,255])。
        cmap: 颜色映射名称 (默认 'viridis')，仅对 2D 数据生效
        aspect: 宽高比。'equal'(默认) 使 X/Y 轴单位长度相同 (图像单元为正方形)；
           'auto' 让图像填满子图框；也可传数值比例
        alpha: 图像整体透明度 (0.0-1.0)
        vmin, vmax: 2D 数据的颜色映射值域 (缺省取数据 min/max)
        origin: 'upper' (默认, 首行在顶部) 或 'lower' (首行在底部)
        interpolation: 插值方法，控制平滑程度。'nearest'/'none'/'antialiased'(默认)
           为块状显示、有明显分界线；'bilinear'/'bicubic' 等对像素做平滑上采样，
           颜色渐变、无硬分界线
        norm: matplotlib Normalize/LogNorm 或 'linear'/'log'。LogNorm 时按对数刻度
           归一化上色，颜色条刻度呈 10 的幂；extent 等: 接受但当前不生效
    """
    # numpy 风格数组（暴露 __array_interface__）直接下沉给 Rust，由底层零拷贝式
    # 读取原始缓冲区；仅对普通 list/tuple 等才在 Python 层递归转换。避免对大图像
    # 调用 .tolist() 生成数百万 Python 浮点对象的开销。
    if not hasattr(x, '__array_interface__'):
        x = _to_list_recursive(x)
    cmap = 'viridis' if cmap is None else cmap
    aspect = 'equal' if aspect is None else aspect
    vmin, vmax = _norm_vminmax(norm, vmin, vmax)
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'imshow'):
        ax.imshow(x, cmap=cmap, aspect=aspect, vmin=vmin, vmax=vmax,
                  alpha=alpha, origin=origin, interpolation=interpolation, norm=norm)
        return _get_figure()
    return _rsplotlib.imshow(x, cmap, aspect, vmin, vmax, alpha, origin,
                             interpolation, _norm_kind(norm))


def imsave(fname, arr, **kwargs):
    """将图像数据直接保存为图片文件 (无坐标轴 / 边距)，兼容 matplotlib.pyplot.imsave。

    输出图片的像素尺寸等于数组尺寸 (N 列 -> 宽, M 行 -> 高)。

    Args:
        fname: 保存的文件名, 相对或绝对路径。格式由 `format` 或文件扩展名决定,
            支持 PNG / JPEG。
        arr: 图像的数组数据。2D 数组经 cmap 上色 (缺省 'viridis'); 3D 数组
            (H, W, 3/4) 视为 RGB(A), 直接按像素颜色写出 (浮点取 [0,1], 整数取 [0,255])。
        cmap: 2D 数据的颜色映射名称 (默认 'viridis')。
        vmin, vmax: 2D 数据的颜色映射值域 (缺省取数据 min/max)。
        origin: 'upper' (默认, 首行在顶部) 或 'lower' (首行在底部)。
        format: 显式指定图片格式 ('png' / 'jpeg')，缺省按扩展名推断。
        dpi: 写入 PNG 的分辨率元数据 (默认 100)。
    """
    # 同 imshow：numpy 风格数组直接下沉给 Rust，避免 .tolist() 开销。
    if not hasattr(arr, '__array_interface__'):
        arr = _to_list_recursive(arr)
    cmap = kwargs.pop('cmap', None) or 'viridis'
    vmin = kwargs.pop('vmin', None)
    vmax = kwargs.pop('vmax', None)
    origin = kwargs.pop('origin', None)
    fmt = kwargs.pop('format', None)
    dpi = kwargs.pop('dpi', None)
    dpi = 100.0 if dpi is None else float(dpi)
    return _rsplotlib.imsave(fname, arr, cmap, vmin, vmax, origin, fmt, dpi)


def imread(fname, format=None):
    """从图像文件读取图像数据，兼容 matplotlib.pyplot.imread。

    返回 ndarray，形状为 (nrows, ncols) 或 (nrows, ncols, nchannels):
    灰度图为 2D (无通道维)；彩色图 nchannels 为 3 (RGB) 或 4 (RGBA)。

    Args:
        fname: 图像文件名或路径 (相对或绝对)。
        format: 图像格式 (如 'png' / 'jpeg')，缺省先按文件内容嗅探，再按扩展名识别。

    按 matplotlib 约定: PNG 返回取值 [0,1] 的浮点数组，其余格式返回取值
    [0,255] 的整数数组。图像解码完全由 Rust 底层实现，返回结果可直接传给 imshow。
    """
    return _rsplotlib.imread(fname, format)


def semilogx(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制 x 轴对数刻度图。"""
    x = _to_seq(x)
    y = _to_seq(y)
    return _rsplotlib.semilogx(x, y, label, color, linestyle, marker, linewidth)


def semilogy(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制 y 轴对数刻度图。"""
    x = _to_seq(x)
    y = _to_seq(y)
    return _rsplotlib.semilogy(x, y, label, color, linestyle, marker, linewidth)


def loglog(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制双对数刻度图。"""
    x = _to_seq(x)
    y = _to_seq(y)
    return _rsplotlib.loglog(x, y, label, color, linestyle, marker, linewidth)


# ==================== 辅助元素 ====================

def text(x, y, s, fontdict=None, **kwargs):
    """添加文本标注。

    Args:
        x, y: 文本位置 (数据坐标)
        s: 文本内容
        fontdict: 字体属性字典 (可选)
        **kwargs: 支持 fontsize, color/c, family, ha, va, rotation, dx, dy 等参数
    """
    fontsize = kwargs.get('fontsize', fontdict.get('fontsize', 12) if fontdict else 12)
    color = kwargs.get('color', fontdict.get('color', 'black') if fontdict else 'black')
    c = kwargs.get('c', None)
    family = kwargs.get('family', None)
    ha = kwargs.get('ha', kwargs.get('horizontalalignment', None))
    va = kwargs.get('va', kwargs.get('verticalalignment', None))
    rotation = kwargs.get('rotation', None)
    dx = kwargs.get('dx', None)
    dy = kwargs.get('dy', None)
    bbox = kwargs.get('bbox', None)
    if not isinstance(s, str):
        s = str(s)
    s = _render_mathtext(s)

    # family 处理：若用户显式指定了字体族名，解析为本地字体文件并注册到
    # plotters 的字体数据库，这样真正驱动文本渲染（而不是被忽略）。
    if family:
        try:
            import os
            from .utils._font_resolver import resolve_font_path
            path = resolve_font_path(family)
            if path is None and os.path.isfile(family):
                path = family  # 也允许直接传文件路径
            if path is not None:
                _rsplotlib.register_sans_serif_font(path)
        except Exception:
            # 字体注册失败不影响绘制（会回退到默认 sans-serif）
            pass

    return _rsplotlib.text(x, y, s, fontsize, color, c, family, ha, va, rotation, dx, dy, bbox)


def axhline(y=0, **kwargs):
    """绘制水平参考线。

    Args:
        y: y 坐标位置
        color: 线颜色
        linestyle: 线型 ('-', '--', ':', '-.')
        linewidth: 线宽
    """
    return _rsplotlib.axhline(y, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def axvline(x=0, **kwargs):
    """绘制垂直参考线。

    Args:
        x: x 坐标位置
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
    """
    return _rsplotlib.axvline(x, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def axhspan(ymin=None, ymax=None, **kwargs):
    """绘制水平区间填充 (在 y 方向高亮一个区间)。

    用法:
        plt.axhspan(0.0, 1.0, color='yellow', alpha=0.3)
        plt.axhspan(ymin=0.0, ymax=1.0, label='Range')

    Args:
        ymin: y 轴下限
        ymax: y 轴上限
        color: 填充颜色 (默认蓝灰色)
        alpha: 透明度 (0.0-1.0, 默认 0.3)
        label: 图例标签
        **kwargs: 其他关键字参数
    """
    ymin = ymin if ymin is not None else kwargs.get('ymin')
    ymax = ymax if ymax is not None else kwargs.get('ymax')
    return _rsplotlib.axhspan(ymin, ymax, kwargs.get('color'), kwargs.get('alpha', 0.3), kwargs.get('label'))


def axvspan(xmin=None, xmax=None, **kwargs):
    """绘制垂直区间填充 (在 x 方向高亮一个区间)。

    用法:
        plt.axvspan(0.0, 1.0, color='yellow', alpha=0.3)
        plt.axvspan(xmin=0.0, xmax=1.0, label='Range')

    Args:
        xmin: x 轴下限
        xmax: x 轴上限
        color: 填充颜色 (默认蓝灰色)
        alpha: 透明度 (0.0-1.0, 默认 0.3)
        label: 图例标签
        **kwargs: 其他关键字参数
    """
    xmin = xmin if xmin is not None else kwargs.get('xmin')
    xmax = xmax if xmax is not None else kwargs.get('xmax')
    return _rsplotlib.axvspan(xmin, xmax, kwargs.get('color'), kwargs.get('alpha', 0.3), kwargs.get('label'))


def axline(xy1, xy2, **kwargs):
    """通过两点绘制任意斜率的直线 (延长到整个绘图区域)。

    用法:
        plt.axline((0, 0), (1, 1), color='red')

    Args:
        xy1: 起点坐标 (x1, y1)
        xy2: 终点坐标 (x2, y2)
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
        **kwargs: 其他关键字参数
    """
    return _rsplotlib.axline(
        tuple(xy1), tuple(xy2),
        kwargs.get('color'), kwargs.get('linestyle'),
        kwargs.get('linewidth'),
    )


# annotate 默认字号：基准 12.0 随其余默认字号一并放大 DEFAULT_FONT_SCALE(=2.0) 倍。
# 与 axes.rs 中默认字号在调用点乘以 DEFAULT_FONT_SCALE 的约定保持一致。
_DEFAULT_ANNOTATE_FONTSIZE = 12.0 * 2.0


def annotate(text, xy, xytext=None, fontsize=None, color='black', arrowprops=None, **kwargs):
    """在指定坐标添加文本标注, 可选带箭头。

    用法:
        plt.annotate('重要点', xy=(1, 2), xytext=(3, 4),
                     arrowprops=dict(arrowstyle='->'))

    Args:
        text: 标注文本内容
        xy: 被标注点的坐标 (数据坐标)
        xytext: 文本放置位置 (数据坐标)。默认与 xy 相同
        fontsize: 字体大小 (默认 None, 采用放大后的默认字号 24.0)
        color: 文本颜色
        arrowprops: 箭头属性字典。None 表示不画箭头; 提供 (哪怕空 dict) 则从
            xytext 绘制箭头指向 xy。支持简单箭头 (width/headwidth/headlength/
            shrink) 与花式箭头 (arrowstyle/connectionstyle/mutation_scale/
            shrinkA/shrinkB 等)。
        **kwargs: 其他关键字参数 (如 xycoords/textcoords/ha/family)，转发给 Axes.annotate
    """
    text = _render_mathtext(text)
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'annotate'):
        ax.annotate(text, xy, xytext, fontsize, color, arrowprops, **kwargs)
        return _get_figure()
    fig, ax = _rsplotlib.subplots()
    ax.annotate(text, xy, xytext, fontsize, color, arrowprops, **kwargs)
    return fig, ax


def hlines(y, xmin=None, xmax=None, **kwargs):
    """在指定 y 位置绘制多条水平线段。

    由 Rust 层批量实现, 避免 Python 级 for 循环。

    Args:
        y: 单个 y 值 或 多个 y 值的列表
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
        **kwargs: 其他关键字参数
    """
    y_arr = _to_list(y) if isinstance(y, (list, tuple)) or hasattr(y, 'tolist') else [y]
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'hlines'):
        ax.hlines(y_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))
        return _get_figure()
    return _rsplotlib.hlines(y_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def vlines(x, ymin=None, ymax=None, **kwargs):
    """在指定 x 位置绘制多条垂直线段 (Rust 层批量实现)。

    Args:
        x: 单个 x 值 或 多个 x 值的列表
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
        **kwargs: 其他关键字参数
    """
    x_arr = _to_list(x) if isinstance(x, (list, tuple)) or hasattr(x, 'tolist') else [x]
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'vlines'):
        ax.vlines(x_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))
        return _get_figure()
    return _rsplotlib.vlines(x_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


# ==================== 配置函数 ====================

def _font_props(fontdict, kwargs):
    """从 fontdict 与关键字参数中提取字体属性 (family, size, color)。

    matplotlib 语义：关键字参数优先于 fontdict；family 支持 family/fontfamily/fontname
    别名，size 支持 size/fontsize，color 支持 color/c。返回 (family, size, color)，
    其中 size 已转为 float 或 None。
    """
    fd = fontdict or {}

    def _pick(*keys):
        for k in keys:
            if kwargs.get(k) is not None:
                return kwargs[k]
        for k in keys:
            if fd.get(k) is not None:
                return fd[k]
        return None

    family = _pick('family', 'fontfamily', 'fontname')
    size = _pick('size', 'fontsize')
    color = _pick('color', 'c')
    size = float(size) if size is not None else None
    return family, size, color


def xlabel(text, fontdict=None, loc=None, **kwargs):
    """设置 x 轴标签文本，并可通过 fontdict / 关键字参数自定义字体属性。

    支持的字体属性 (fontdict 的键或直接关键字参数, 关键字参数优先):
        family / fontfamily / fontname: 字体族名 (如 'Courier'、'STHeiti Light' 等)
        size / fontsize: 字号 (points)
        color: 文本颜色 (如 'r'、'#ff0000'、'SeaGreen')

    loc: 标签水平位置，可选 'left'、'center'、'right'，默认 'center'。

    用法:
        plt.xlabel("x - label")
        plt.xlabel("x 轴", fontdict={"family": "STHeiti Light", "size": 16, "color": "b"})
        plt.xlabel("x 轴", loc="left")
    """
    family, size, color = _font_props(fontdict, kwargs)
    return _rsplotlib.xlabel(_render_mathtext(text), color, size, family, loc)


def ylabel(text, fontdict=None, loc=None, **kwargs):
    """设置 y 轴标签文本，并可通过 fontdict / 关键字参数自定义字体属性。

    支持的字体属性同 xlabel（family / size / color，关键字参数优先于 fontdict）。

    loc: 标签垂直位置，可选 'bottom'、'center'、'top'，默认 'center'。

    用法:
        plt.ylabel("y - label")
        plt.ylabel("y 轴", fontdict={"family": "STHeiti Light", "size": 16, "color": "g"})
        plt.ylabel("y 轴", loc="top")
    """
    family, size, color = _font_props(fontdict, kwargs)
    return _rsplotlib.ylabel(_render_mathtext(text), color, size, family, loc)


def title(label, fontdict=None, loc=None, **kwargs):
    """设置图表标题文本，并可通过 fontdict / 关键字参数自定义字体属性。

    支持的字体属性 (fontdict 的键或直接关键字参数, 关键字参数优先):
        family / fontfamily / fontname: 字体族名 (如 'Courier'、'Times New Roman'、
            'SimHei' 等)
        size / fontsize: 字号 (points)
        color: 文本颜色 (如 'r'、'#ff0000'、'SeaGreen')

    loc: 标题水平位置，可选 'left'、'center'、'right'，默认 'center'。
    pad: 标题与数据区顶部的间距（points，默认 5.0）。

    用法:
        plt.title("标题")
        plt.title("标题", fontdict={"family": "Courier", "size": 18, "color": "red"})
        plt.title("标题", fontsize=18, color='b')
        plt.title("标题", loc="left")
        plt.title("第一行\n第二行", pad=0.02)
    """
    family, size, color = _font_props(fontdict, kwargs)
    fd = fontdict or {}
    pad = kwargs.get('pad') or fd.get('pad')
    try:
        pad = None if pad is None else float(pad)
    except (TypeError, ValueError):
        pad = None
    return _rsplotlib.title(_render_mathtext(label), color, size, family, loc, pad)


def suptitle(t, **kwargs):
    """设置整个图形的总标题（居中显示在所有子图上方）。

    Args:
        t: 标题文本

    用法:
        plt.suptitle("总标题")
    """
    return _rsplotlib.gcf().suptitle(_render_mathtext(str(t)))


def grid(visible=True, **kwargs):
    """显示或隐藏网格线。

    Args:
        visible: 是否显示 (默认 True)
        color/c: 线颜色
        linestyle/ls: 线型
        linewidth/lw: 线宽
        axis: 坐标轴 ('x', 'y', 或 'both')
    """
    c = kwargs.get('color') or kwargs.get('c')
    ls = kwargs.get('linestyle') or kwargs.get('ls')
    lw = kwargs.get('linewidth') or kwargs.get('lw')
    axis = kwargs.get('axis')
    return _rsplotlib.grid(visible, c, ls, lw, axis)


_LOC_MAP = {
    0: 'best',
    1: 'upper right',
    2: 'upper left',
    3: 'lower left',
    4: 'lower right',
    5: 'right',
    6: 'center left',
    7: 'center right',
    8: 'lower center',
    9: 'upper center',
    10: 'center',
}


def legend(loc='best', **kwargs):
    """显示图例 (需要 plot 时设置 label 参数)。

    Args:
        loc: 图例位置 ('best', 'upper right', 'upper left', 'lower left',
              'lower right', 'upper center', 'lower center',
              'center left', 'center right', 'center')，也支持整数 0-10
        facecolor: 图例框背景色 (颜色名或 '#RRGGBB')，默认沿用半透明白底
        framealpha: 图例框背景不透明度 (0-1)，默认 0.85
        edgecolor: 图例框边框色，默认浅灰
        fontsize: 图例文字字号 (point)，默认 11.0
    """
    if isinstance(loc, int):
        loc = _LOC_MAP.get(loc, 'best')
    facecolor, framealpha, edgecolor, fontsize = _legend_frame_kwargs(kwargs)
    return _rsplotlib.legend(loc, facecolor, framealpha, edgecolor, fontsize)


def _legend_frame_kwargs(kwargs):
    """从 kwargs 提取并规范化图例框样式 (facecolor, framealpha, edgecolor, fontsize)。

    非字符串颜色一律忽略 (置 None)，交由后端使用默认值。
    """
    facecolor = kwargs.get('facecolor')
    edgecolor = kwargs.get('edgecolor')
    framealpha = kwargs.get('framealpha')
    fontsize = kwargs.get('fontsize')
    if not isinstance(facecolor, str):
        facecolor = None
    if not isinstance(edgecolor, str):
        edgecolor = None
    try:
        framealpha = None if framealpha is None else float(framealpha)
    except (TypeError, ValueError):
        framealpha = None
    try:
        fontsize = None if fontsize is None else float(fontsize)
    except (TypeError, ValueError):
        fontsize = None
    return facecolor, framealpha, edgecolor, fontsize


def xlim(left=None, right=None, **kwargs):
    """设置 x 轴显示范围。

    Args:
        left: x 轴最小值
        right: x 轴最大值
    """
    return _rsplotlib.xlim(left, right)


def ylim(bottom=None, top=None, **kwargs):
    """设置 y 轴显示范围。

    Args:
        bottom: y 轴最小值
        top: y 轴最大值
    """
    return _rsplotlib.ylim(bottom, top)


def xticks(ticks=None, labels=None, **kwargs):
    """设置 x 轴刻度。

    Args:
        ticks: 刻度位置列表
        labels: 刻度标签列表
    """
    ticks = _to_list(ticks)
    return _rsplotlib.xticks(ticks, labels)


def yticks(ticks=None, labels=None, **kwargs):
    """设置 y 轴刻度。

    Args:
        ticks: 刻度位置列表
        labels: 刻度标签列表
    """
    ticks = _to_list(ticks)
    return _rsplotlib.yticks(ticks, labels)


def xscale(scale, **kwargs):
    """设置 x 轴刻度类型 ('linear', 'log', 'logit', 'symlog' 等)。"""
    return _rsplotlib.xscale(scale)


def yscale(scale, **kwargs):
    """设置 y 轴刻度类型。"""
    return _rsplotlib.yscale(scale)


def margins(x_margin=None, y_margin=None, **kwargs):
    """设置自动缩放的边距。"""
    return _rsplotlib.margins(x_margin, y_margin)


def box(on=None):
    """设置坐标轴边框。"""
    return _rsplotlib.box_(on)


def minorticks_on():
    """显示次要刻度。"""
    return _rsplotlib.minorticks_on()


def minorticks_off():
    """隐藏次要刻度。"""
    return _rsplotlib.minorticks_off()


# ==================== 子图与布局 ====================

class _AxesArray:
    """轻量 Axes 网格容器，模拟 numpy 数组的常用索引方式。

    使 plt.subplots 的返回值支持 axs[i, j] 元组索引、axs[i] 行/元素索引、
    迭代、.flat / .flatten() / .ravel()，且不依赖任何第三方数组库。

    内部以行主序扁平 list 保存 Axes；shape 为 (n,) 表示一维、(nrows, ncols)
    表示二维。
    """
    __slots__ = ('_data', 'shape')

    def __init__(self, data, shape):
        self._data = list(data)
        self.shape = shape

    @property
    def ndim(self):
        return len(self.shape)

    @property
    def size(self):
        return len(self._data)

    @property
    def flat(self):
        """按行主序遍历所有 Axes 的迭代器。"""
        return iter(self._data)

    def flatten(self):
        """返回按行主序排列的一维 list。"""
        return list(self._data)

    ravel = flatten

    def __len__(self):
        return self.shape[0]

    def __iter__(self):
        # 一维：逐个产出 Axes；二维：逐行产出子 _AxesArray（与 numpy 一致）。
        if self.ndim == 1:
            return iter(self._data)
        ncols = self.shape[1]
        return (
            _AxesArray(self._data[r * ncols:(r + 1) * ncols], (ncols,))
            for r in range(self.shape[0])
        )

    def __getitem__(self, key):
        if isinstance(key, tuple):
            if self.ndim != 2 or len(key) != 2:
                raise IndexError('二维索引仅适用于二维 Axes 数组')
            nrows, ncols = self.shape
            r, c = key
            
            # 处理行索引
            if isinstance(r, slice):
                row_indices = range(nrows)[r]
            else:
                r = r % nrows
                row_indices = [r]
            
            # 处理列索引
            if isinstance(c, slice):
                col_indices = range(ncols)[c]
            else:
                c = c % ncols
                col_indices = [c]
            
            # 收集所有匹配的 Axes
            result = []
            for r_idx in row_indices:
                for c_idx in col_indices:
                    result.append(self._data[r_idx * ncols + c_idx])
            
            # 根据结果形状返回
            if len(row_indices) == 1 and len(col_indices) == 1:
                return result[0]
            elif len(row_indices) == 1:
                return _AxesArray(result, (len(col_indices),))
            elif len(col_indices) == 1:
                return _AxesArray(result, (len(row_indices),))
            else:
                return _AxesArray(result, (len(row_indices), len(col_indices)))
        
        if self.ndim == 1:
            if isinstance(key, slice):
                indices = range(self.shape[0])[key]
                result = [self._data[i] for i in indices]
                return _AxesArray(result, (len(result),))
            return self._data[key]
        
        # 二维单整数索引返回对应行（子 _AxesArray）。
        nrows, ncols = self.shape
        if isinstance(key, slice):
            row_indices = range(nrows)[key]
            result = []
            for r_idx in row_indices:
                result.extend(self._data[r_idx * ncols:(r_idx + 1) * ncols])
            return _AxesArray(result, (len(row_indices), ncols))
        
        if key < 0:
            key += nrows
        return _AxesArray(self._data[key * ncols:(key + 1) * ncols], (ncols,))

    def __repr__(self):
        return f'AxesArray(shape={self.shape})'


def _apply_layout(fig, kwargs):
    """把 matplotlib 的 layout 关键字转成后端智能均匀边距开关。

    layout='constrained'/'tight'/'compressed'（或已弃用的 constrained_layout=True /
    tight_layout=True）时，启用 constrained 智能布局：渲染时按各边装饰范围反解四周
    边距，使图四周留白均匀、适中（保持 figsize 不变）。其余取值不启用。
    """
    setter = getattr(fig, 'set_constrained_layout', None)
    if setter is None:
        return
    layout = kwargs.get('layout')
    if isinstance(layout, str) and layout.lower() in ('constrained', 'tight', 'compressed'):
        setter(True)
    elif kwargs.get('constrained_layout') or kwargs.get('tight_layout'):
        setter(True)


def subplots(nrows=1, ncols=1, figsize=None, dpi=None, squeeze=True, **kwargs):
    """创建子图网格 (Figure + Axes)。

    用法:
        fig, ax = plt.subplots()                    # 单图 (1x1)
        fig, axes = plt.subplots(2, 2)             # 2x2 网格
        fig, axes = plt.subplots(1, 2, figsize=(10, 5))  # 自定义尺寸
        fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})  # 极坐标

    Args:
        nrows: 子图行数 (默认 1)
        ncols: 子图列数 (默认 1)
        figsize: 图的尺寸 (width, height), 单位英寸
        dpi: 分辨率 (每英寸点数)
        squeeze: 是否压缩返回的 Axes 数组维度 (默认 True), 与 matplotlib 一致
        **kwargs: 其他关键字参数，包括 subplot_kw 用于传递子图参数如 projection

    Returns:
        与 matplotlib 一致的返回值 (squeeze=True 时):
        - 1x1: (Figure, Axes)
        - 1xN 或 Nx1: (Figure, 一维 ndarray[Axes])
        - MxN: (Figure, 二维 ndarray[Axes]), 支持 axs[i, j] 索引
    """
    gridspec_kw = kwargs.get('gridspec_kw') or {}
    subplot_kw = kwargs.get('subplot_kw') or {}
    width_ratios = kwargs.get('width_ratios')
    if width_ratios is None:
        width_ratios = gridspec_kw.get('width_ratios')
    height_ratios = kwargs.get('height_ratios')
    if height_ratios is None:
        height_ratios = gridspec_kw.get('height_ratios')
    width_ratios = _to_list(width_ratios) if width_ratios is not None else None
    height_ratios = _to_list(height_ratios) if height_ratios is not None else None

    layout = kwargs.get('layout')
    projection = subplot_kw.get('projection')
    result = _rsplotlib.subplots(nrows, ncols, figsize, dpi, width_ratios, height_ratios, layout, projection)
    fig = result[0]

    # gridspec_kw={'wspace':.., 'hspace':..} 与 matplotlib 一致地控制子图间距，
    # 复用 subplots_adjust 的存储路径（在渲染阶段覆盖默认/启发式间距）。
    if gridspec_kw:
        ws = gridspec_kw.get('wspace')
        hs = gridspec_kw.get('hspace')
        if ws is not None or hs is not None:
            fig.subplots_adjust(wspace=ws, hspace=hs)

    if nrows == 1 and ncols == 1:
        single = result[1]
        if squeeze:
            return fig, single
        flat = [single]
    else:
        flat = list(result[1])

    # 依 matplotlib 的 squeeze 规则确定返回形状：单行 / 单列压成一维，其余保持二维。
    # 用模块自带的 _AxesArray 提供 axs[i, j] 索引，不依赖任何第三方数组库。
    if squeeze and nrows == 1:
        return fig, _AxesArray(flat, (ncols,))
    if squeeze and ncols == 1:
        return fig, _AxesArray(flat, (nrows,))
    return fig, _AxesArray(flat, (nrows, ncols))


def _mosaic_track_edges(n, ratios, space):
    """把 [0,1] 分成 n 条轨道，相邻轨道间留 `space` 比例间隙。

    返回长度 n 的 [(start, size), ...]（沿正方向）。`ratios` 为各轨道相对尺寸
    (None 视为等分)；`space` 为间隙占「平均轨道尺寸」的比例，与 matplotlib 的
    wspace/hspace 语义一致 (等分时退化为后端 grid_position 的公式)。
    """
    if n <= 0:
        return []
    if ratios is None:
        ratios = [1.0] * n
    ratios = [float(r) for r in ratios]
    if len(ratios) != n:
        raise ValueError(f"ratios 长度 {len(ratios)} 与轨道数 {n} 不符")
    total_r = sum(ratios)
    if total_r <= 0:
        raise ValueError("ratios 之和必须为正")
    gap_r = space * (total_r / n)           # 间隙的 ratio 单位
    unit = 1.0 / (total_r + (n - 1) * gap_r)  # ratio 单位 -> [0,1] 分数
    edges = []
    cursor = 0.0
    for r in ratios:
        size = r * unit
        edges.append((cursor, size))
        cursor += size + gap_r * unit
    return edges


# subplot_mosaic 默认间距（未经 gridspec_kw 指定时）：与后端规则网格默认一致，
# 足以容纳每个子图的标题 + 刻度值而不相互重叠。
_MOSAIC_WSPACE = 0.3
_MOSAIC_HSPACE = 0.55


def subplot_mosaic(mosaic, *, sharex=False, sharey=False,
                   width_ratios=None, height_ratios=None,
                   empty_sentinel='.', subplot_kw=None, gridspec_kw=None,
                   per_subplot_kw=None, figsize=None, dpi=None, **fig_kw):
    """根据 ASCII 布局或嵌套标签列表创建命名子图 (matplotlib subplot_mosaic 兼容)。

    参数:
        mosaic: 可视化布局。可为多行字符串 (每字符一列、每行一行) 或标签的二维列表。
            相同标签在网格中的矩形包围盒合并为一个跨行/跨列的子图，`empty_sentinel`
            (默认 '.') 表示留空。
        sharex, sharey: 接受以兼容 matplotlib 签名 (当前后端各子图坐标轴相互独立)。
        width_ratios: 长度为 ncols 的列宽相对比例；等价于 gridspec_kw={'width_ratios': ...}。
        height_ratios: 长度为 nrows 的行高相对比例；等价于 gridspec_kw={'height_ratios': ...}。
        empty_sentinel: 表示「留空」的条目，默认 '.'。
        subplot_kw: 传给每个子图的关键字参数 (经 Axes.set 应用)。
        gridspec_kw: 网格参数，识别 wspace / hspace / width_ratios / height_ratios。
        per_subplot_kw: {标签 或 标签元组: kwargs} 的按图覆盖 (优先级高于 subplot_kw)；
            字符串 mosaic 下键可用多字符串 (如 "AB" 等价于 ('A','B'))。
        figsize, dpi, **fig_kw: 传给 figure() (如 layout='constrained')。

    返回:
        (Figure, {label: Axes})。各子图之间按 wspace/hspace 自动留出间距。
    """
    # —— 解析 mosaic 为二维标签网格 ——
    if isinstance(mosaic, str):
        lines = [ln.strip() for ln in mosaic.strip().splitlines()]
        rows = [list(ln) for ln in lines if ln]
    else:
        rows = []
        for r in mosaic:
            if any(isinstance(c, (list, tuple)) for c in r):
                raise NotImplementedError("subplot_mosaic 暂不支持嵌套布局")
            rows.append(list(r))
    nrows = len(rows)
    ncols = max((len(r) for r in rows), default=0)
    if nrows == 0 or ncols == 0:
        raise ValueError("mosaic 不能为空")
    # 列表输入允许行长不齐：短行用 empty_sentinel 补齐。
    for r in rows:
        if len(r) < ncols:
            r.extend([empty_sentinel] * (ncols - len(r)))

    # —— 收集每个标签的矩形包围盒 (row_start, row_end, col_start, col_end) ——
    def _is_empty(label):
        return label is None or label == empty_sentinel

    boxes = {}
    order = []
    for ri, row in enumerate(rows):
        for ci, label in enumerate(row):
            if _is_empty(label):
                continue
            if label not in boxes:
                boxes[label] = [ri, ri + 1, ci, ci + 1]
                order.append(label)
            else:
                b = boxes[label]
                b[0] = min(b[0], ri)
                b[1] = max(b[1], ri + 1)
                b[2] = min(b[2], ci)
                b[3] = max(b[3], ci + 1)

    # —— 网格间距 / 比例：显式参数优先于 gridspec_kw ——
    gridspec_kw = dict(gridspec_kw or {})
    if width_ratios is None:
        width_ratios = gridspec_kw.get('width_ratios')
    if height_ratios is None:
        height_ratios = gridspec_kw.get('height_ratios')
    wspace = gridspec_kw.get('wspace')
    hspace = gridspec_kw.get('hspace')
    if wspace is None:
        wspace = _MOSAIC_WSPACE
    if hspace is None:
        hspace = _MOSAIC_HSPACE

    col_edges = _mosaic_track_edges(ncols, width_ratios, wspace)   # 从左向右 (x)
    row_edges = _mosaic_track_edges(nrows, height_ratios, hspace)  # 从上向下 (自顶部量)

    # —— per_subplot_kw 归一化为 {label: kwargs} ——
    psk = {}
    if per_subplot_kw:
        for k, v in per_subplot_kw.items():
            if isinstance(k, tuple):
                keys = k
            elif isinstance(mosaic, str) and isinstance(k, str) and len(k) > 1:
                keys = tuple(k)
            else:
                keys = (k,)
            for kk in keys:
                psk.setdefault(kk, {}).update(v)

    fig = figure(figsize=figsize, dpi=dpi, **fig_kw)
    axd = {}
    for label in order:
        rs, re, cs, ce = boxes[label]
        left = col_edges[cs][0]
        right = col_edges[ce - 1][0] + col_edges[ce - 1][1]
        top = 1.0 - row_edges[rs][0]
        bottom = 1.0 - (row_edges[re - 1][0] + row_edges[re - 1][1])
        ax = fig.add_axes(left, bottom, right - left, top - bottom)
        merged = {}
        if subplot_kw:
            merged.update(subplot_kw)
        if label in psk:
            merged.update(psk[label])
        if merged:
            ax.set(**merged)
        axd[label] = ax
    return fig, axd


def subplot(*args, **kwargs):
    """在当前 figure 中添加一个子图 (Axes)，兼容 matplotlib 的调用签名。

    调用签名:
        subplot(nrows, ncols, index, **kwargs)
        subplot(pos, **kwargs)   # pos 为三位整数, 如 subplot(131) 等价于 subplot(1, 3, 1)
        subplot(**kwargs)        # 无位置参数, 默认为 (1, 1, 1)

    *args 描述子图位置, 可为下列之一:
        - 三个整数 (nrows, ncols, index): 子图占据 nrows 行 ncols 列网格中的 index
          位置, index 从左上角的 1 开始向右递增。index 也可为二元组 (first, last)
          (基于 1, 含 last), 表示子图跨越 first 到 last 的格子, 例如
          subplot(3, 1, (1, 2)) 创建一个跨越上部 2/3 的子图。
        - 一个三位整数 (如 131 等价于 1, 3, 1), 仅在子图数不超过 9 时可用。
        - 一个 SubplotSpec。

    关键字参数:
        projection: 投影类型。当前后端仅支持默认的直角坐标 (rectilinear),
                    其他值被接受但不生效。
        polar (bool): 为 True 时等价于 projection='polar' (当前后端不支持, 接受但不生效)。
        sharex, sharey: 与之共享 x / y 轴的 Axes (当前接受但不生效)。
        label (str): 返回 Axes 的标签。

    Returns:
        Axes: 新建或已存在的子图。
    """
    polar = kwargs.pop('polar', False)
    projection = kwargs.pop('projection', None)
    kwargs.pop('sharex', None)
    kwargs.pop('sharey', None)
    label = kwargs.pop('label', None)
    if polar and projection is None:
        projection = 'polar'

    if len(args) == 0:
        nrows, ncols, index = 1, 1, 1
    elif len(args) == 1:
        pos = args[0]
        if hasattr(pos, 'rowStart'):  # SubplotSpec
            nrows, ncols, index = _spec_to_grid(pos)
        elif isinstance(pos, int) and not isinstance(pos, bool):
            if not 100 <= pos <= 999:
                raise ValueError(f"整数子图参数必须是三位数 (如 131)，收到 {pos}")
            nrows, ncols, index = pos // 100, (pos // 10) % 10, pos % 10
        else:
            raise TypeError("subplot() 单参数形式仅支持三位整数 (如 131) 或 SubplotSpec")
    elif len(args) == 3:
        nrows, ncols, index = args
    else:
        raise TypeError(
            f"subplot() 需要 0 个、1 个 (三位整数 / SubplotSpec) 或 3 个位置参数，收到 {len(args)}"
        )

    # index 既可为整数（单格子）也可为 (first, last) 二元组（跨格子），原生 subplot 均支持。
    ax = _rsplotlib.subplot(nrows, ncols, index)[1]
    _apply_axes_label(ax, label)
    return ax


def _spec_to_grid(spec):
    """将 SubplotSpec 转换为 (nrows, ncols, (first, last))，供原生 subplot 使用。

    SubplotSpec 用 0 基、rowStop/colStop 为开区间上界的方式描述网格跨度；
    这里换算为基于 1、含端点的线性 (first, last) 索引。
    """
    nrows = int(spec.numRows)
    ncols = int(spec.numCols)
    first = int(spec.rowStart) * ncols + int(spec.colStart) + 1
    last = (int(spec.rowStop) - 1) * ncols + (int(spec.colStop) - 1) + 1
    return nrows, ncols, (first, last)


def tight_layout(**kwargs):
    """自动调整子图布局, 避免标签重叠。"""
    return _rsplotlib.tight_layout()


def subplots_adjust(left=None, right=None, bottom=None, top=None, wspace=None, hspace=None):
    """调整子图布局参数"""
    fig = _get_figure()
    if fig is not None:
        fig.subplots_adjust(left, right, bottom, top, wspace, hspace)


def set_size(width, height):
    """设置图形像素尺寸。"""
    return _rsplotlib.set_size(width, height)


def twinx():
    """创建共享 x 轴的双 y 轴。"""
    return _rsplotlib.twinx()


def twiny():
    """创建共享 y 轴的双 x 轴。"""
    return _rsplotlib.twiny()


# ==================== 图形控制 ====================

_FIGURES = {}
_FIGURE_COUNTER = 1


def get_fignums():
    """返回当前所有图形的编号列表。"""
    return [num for num, fig in _FIGURES.items() if len(fig.get_axes()) > 0]


def figure(num=None, figsize=None, dpi=None, **kwargs):
    """创建新的 Figure 对象。

    Args:
        num: 图形编号，如果已存在则返回该图形
        figsize: (width, height) 英寸数
        dpi: 分辨率
        **kwargs: 其他关键字参数

    Returns:
        Figure 对象
    """
    global _FIGURE_COUNTER

    if num is not None and num in _FIGURES:
        return _FIGURES[num]

    if figsize is None:
        figsize = tuple(_get_rcparams().get('figure.figsize', DEFAULT_FIGSIZE))
    layout = kwargs.get('layout')
    fig = _rsplotlib.figure(figsize=figsize, dpi=dpi, layout=layout)

    if num is None:
        num = _FIGURE_COUNTER
        _FIGURE_COUNTER += 1
    _FIGURES[num] = fig

    return fig


def axes(arg=None, **kwargs):
    """向当前图形添加一个坐标区并返回它 (matplotlib plt.axes 兼容)。

    arg 为 None 时在当前图形上创建一个全幅坐标区 (等价于 add_subplot())；
    没有当前图形则新建一个。传入 rect [left, bottom, width, height] 的定位形式
    当前不支持，按全幅处理。其余关键字作为坐标区属性 (如 xlabel/ylabel/title) 应用。
    """
    fig = _get_figure()
    if fig is None:
        fig = figure()
    ax = fig.add_subplot()
    if kwargs:
        ax.set(**kwargs)
    return ax


def savefig(fname, **kwargs):
    """保存图形到文件。

    支持的文件格式:
        - .png: 便携式网络图形 (位图)
        - .jpg: JPEG 图像
        - .svg: 可缩放矢量图形

    用法:
        plt.savefig('figure.png')
        plt.savefig('figure.png', dpi=300)   # 高分辨率
        plt.savefig('figure.svg')

    Args:
        fname: 输出文件名 (含扩展名)
        dpi: 分辨率 (每英寸点数, 默认与创建时一致)。
             对于高清晰度出版物, 使用 300 或更高。
        **kwargs: 其他关键字参数
    """
    dpi = kwargs.get('dpi')
    fig = _get_figure()
    if fig is not None:
        if dpi is not None:
            fig.savefig(fname, dpi)
        else:
            fig.savefig(fname)
        return
    # 无 Figure 时回退到模块级
    if dpi is not None:
        _rsplotlib.savefig(fname, dpi)
    else:
        _rsplotlib.savefig(fname)


def show(**kwargs):
    """在默认应用中显示图形。无当前 figure 时静默返回（与 matplotlib 一致）。"""
    if _get_figure() is None:
        return None
    return _rsplotlib.show()


def isinteractive():
    """交互模式状态。rsplotlib 使用非交互 (Agg) 后端，恒为 False（与 matplotlib 非交互模式一致）。"""
    return False


def draw(*args, **kwargs):
    """重绘当前 figure。非交互后端下为空操作（实际渲染在 savefig/show 时进行）。"""
    return None


def gca(*args, **kwargs):
    """获取当前 Axes；无当前 figure/axes 时自动新建（与 matplotlib 一致）。"""
    try:
        return _rsplotlib.gca()
    except Exception:
        pass
    fig = _get_figure()
    if fig is None:
        fig = figure(args)
    return fig.add_subplot()


def gcf(**kwargs):
    """获取当前 Figure。"""
    return _rsplotlib.gcf()


def cla():
    """清空当前 Axes 内容。"""
    return _rsplotlib.cla()


def clf():
    """清空当前 Figure 内容 (清除所有子图)。"""
    return _rsplotlib.clf()


def close(fig=None):
    """关闭当前 Figure。

    Args:
        fig: 图形或 'all' (兼容 matplotlib)
    """
    return _rsplotlib.close()


def axis(arg=None, **kwargs):
    """获取或设置坐标轴属性，兼容 matplotlib.pyplot.axis。

    调用形式:
        axis()                          -> 返回当前 (xmin, xmax, ymin, ymax)
        axis((xmin, xmax, ymin, ymax))  -> 一次性设置四个轴限 (也接受 list)
        axis('off')  / axis(False)      -> 隐藏坐标轴装饰
        axis('on')   / axis(True)       -> 显示坐标轴装饰
        axis('equal') / axis('scaled')  -> 等比例缩放
        axis(xmin=.., xmax=.., ymin=.., ymax=..)  -> 关键字设置轴限

    返回当前的 (xmin, xmax, ymin, ymax)（无法获取时返回 None）。
    """
    ax = _get_axes()
    if isinstance(arg, (list, tuple)):
        # 序列形式: [xmin, xmax, ymin, ymax]
        if len(arg) != 4:
            raise ValueError(
                "axis(): 序列参数必须为 (xmin, xmax, ymin, ymax) 四个值"
            )
        xmin, xmax, ymin, ymax = arg
        xlim(xmin, xmax)
        ylim(ymin, ymax)
    elif arg is False or arg == 'off':
        if ax is not None:
            try:
                ax._axis_off()
            except Exception:
                pass
    elif arg is True or arg == 'on':
        if ax is not None and hasattr(ax, '_axis_on'):
            ax._axis_on()
    elif arg in ('equal', 'scaled', 'image', 'square'):
        if ax is not None:
            ax.set_aspect('equal')
    # 'tight' / 'auto' 等自动缩放选项：数据本已填充绘图框，保持当前行为。

    # 关键字形式: axis(xmin=.., xmax=.., ymin=.., ymax=..)
    if kwargs:
        xmn, xmx = kwargs.get('xmin'), kwargs.get('xmax')
        ymn, ymx = kwargs.get('ymin'), kwargs.get('ymax')
        if xmn is not None or xmx is not None:
            xlim(xmn, xmx)
        if ymn is not None or ymx is not None:
            ylim(ymn, ymx)

    # 返回当前范围 (xmin, xmax, ymin, ymax)
    if ax is not None and hasattr(ax, 'get_xlim') and hasattr(ax, 'get_ylim'):
        try:
            x0, x1 = ax.get_xlim()
            y0, y1 = ax.get_ylim()
            return (x0, x1, y0, y1)
        except Exception:
            return None
    return None


def _apply_colorbar(target, kwargs):
    """把 matplotlib colorbar kwargs 解析后应用到目标 Axes。

    支持 location / orientation / shrink / aspect / pad / fraction / label / extend /
    ticks / format；其余参数（cax / use_gridspec / anchor / panchor / extendfrac /
    extendrect / drawedges / boundaries / values / spacing 等）接受但当前不生效。
    """
    if target is None:
        return None
    ex = getattr(target, 'enable_colorbar_ex', None)
    if ex is None:
        if hasattr(target, 'enable_colorbar'):
            target.enable_colorbar()
        return None

    def _f(v):
        try:
            return float(v)
        except (TypeError, ValueError):
            return None

    ticks = kwargs.get('ticks')
    ticks_list = None
    if ticks is not None:
        seq = _to_list(ticks)
        if isinstance(seq, (list, tuple)):
            ticks_list = [float(t) for t in seq if _f(t) is not None]

    fmt = kwargs.get('format')
    location = kwargs.get('location')
    orientation = kwargs.get('orientation')
    extend = kwargs.get('extend')
    label = kwargs.get('label')

    ex(
        location=str(location) if location is not None else None,
        orientation=str(orientation) if orientation is not None else None,
        shrink=_f(kwargs.get('shrink')),
        aspect=_f(kwargs.get('aspect')),
        pad=_f(kwargs.get('pad')),
        fraction=_f(kwargs.get('fraction')),
        label=str(label) if label is not None else None,
        extend=str(extend) if extend is not None else None,
        ticks=ticks_list,
        format=fmt if isinstance(fmt, str) else None,
    )
    return None


def colorbar(mappable=None, cax=None, ax=None, **kwargs):
    """在目标坐标区上添加颜色条。

    颜色条基于最近一次可映射绘制（scatter 数值 c + cmap，或 imshow / pcolormesh /
    contourf）记录的 (cmap, vmin, vmax) 信息渲染。若此前没有可映射绘制，则按
    viridis / [0,1] 兜底。支持 location/orientation/shrink/aspect/pad/fraction/
    label/extend/ticks/format 等参数（见 `_apply_colorbar`）。
    """
    target = ax
    if target is None and mappable is not None:
        target = getattr(mappable, 'axes', None)
    if target is None:
        target = _get_axes()
    _apply_colorbar(target, kwargs)
    return None


def get_cmap(name=None, lut=None):
    """获取颜色映射 (占位实现)。"""
    return name


# ==================== Axes 类补丁 ====================

def _patch_axes_get_gridspec():
    """为 Rust Axes 类添加 get_gridspec() 支持。
    
    这是 matplotlib 兼容接口，用于获取子图所在的网格布局对象。
    返回的 GridSpec 支持 gs[row, col] 和 gs[row_start:row_end, col_start:col_end] 语法。
    """
    from . import rsplotlib as _rs
    from .layout.gridspec import GridSpec
    
    def _get_gridspec(self):
        # 获取 Axes 所属的 Figure
        try:
            # 获取当前 figure
            fig = _get_figure()
            if fig is None:
                return None
            
            # 检查 figure 是否有 nrows 和 ncols 方法
            if hasattr(fig, 'nrows') and callable(fig.nrows):
                nrows = fig.nrows()
            else:
                nrows = 1
            
            if hasattr(fig, 'ncols') and callable(fig.ncols):
                ncols = fig.ncols()
            else:
                ncols = 1
            
            # 获取 figure 的边界信息
            if hasattr(fig, 'subplot_left') and callable(fig.subplot_left):
                left = fig.subplot_left()
            else:
                left = None
            
            if hasattr(fig, 'subplot_right') and callable(fig.subplot_right):
                right = fig.subplot_right()
            else:
                right = None
            
            if hasattr(fig, 'subplot_bottom') and callable(fig.subplot_bottom):
                bottom = fig.subplot_bottom()
            else:
                bottom = None
            
            if hasattr(fig, 'subplot_top') and callable(fig.subplot_top):
                top = fig.subplot_top()
            else:
                top = None
            
            # 获取间距信息
            if hasattr(fig, 'subplot_wspace') and callable(fig.subplot_wspace):
                wspace = fig.subplot_wspace()
            else:
                wspace = None
            
            if hasattr(fig, 'subplot_hspace') and callable(fig.subplot_hspace):
                hspace = fig.subplot_hspace()
            else:
                hspace = None
            
            # 获取宽度和高度比例
            if hasattr(fig, 'width_ratios') and callable(fig.width_ratios):
                width_ratios = fig.width_ratios()
            else:
                width_ratios = None
            
            if hasattr(fig, 'height_ratios') and callable(fig.height_ratios):
                height_ratios = fig.height_ratios()
            else:
                height_ratios = None
            
            # 创建一个 GridSpec 对象，使用 figure 的边界信息
            gs = GridSpec(nrows, ncols, left=left, right=right, bottom=bottom, top=top, wspace=wspace, hspace=hspace, width_ratios=width_ratios, height_ratios=height_ratios)
            return gs
        except Exception:
            # 如果获取失败，返回一个默认的 GridSpec
            return GridSpec(1, 1)
    
    _rs.Axes.get_gridspec = _get_gridspec


def _patch_axes_remove():
    """为 Rust Axes 类添加 remove() 支持。
    
    这是 matplotlib 兼容接口，用于从 Figure 中移除当前 Axes。
    """
    from . import rsplotlib as _rs
    
    def _remove(self):
        # 从 Figure 中移除当前 Axes
        try:
            # 尝试获取当前 figure
            fig = _get_figure()
            if fig is None:
                return
            
            # 检查当前 axes 是否在 figure 中
            if self in fig.get_axes():
                # 使用 figure 的 remove_axes 方法
                if hasattr(fig, 'remove_axes'):
                    fig.remove_axes(self)
        except Exception:
            pass
    
    _rs.Axes.remove = _remove


# ==================== Figure 类补丁 ====================

def _patch_figure_add_subplot():
    """为 Rust Figure 类添加 add_subplot(nrows, ncols, index) 支持。"""
    from . import rsplotlib as _rs

    _orig_add_subplot = _rs.Figure.add_subplot

    def _add_subplot(self, *args, **kwargs):
        if len(args) == 0:
            # matplotlib: add_subplot() 等价于 add_subplot(1, 1, 1)
            args = (1, 1, 1)
        if len(args) == 1:
            ax = _orig_add_subplot(self, args[0])
        elif len(args) == 3:
            nrows, ncols, index = args
            if isinstance(index, tuple):
                indices = [i - 1 for i in index]
                row_start = indices[0] // ncols
                row_end = indices[-1] // ncols + 1
                col_start = indices[0] % ncols
                col_end = indices[-1] % ncols + 1
            else:
                index_0 = index - 1
                row_start = index_0 // ncols
                row_end = row_start + 1
                col_start = index_0 % ncols
                col_end = col_start + 1
            from .layout.gridspec import SubplotSpec
            spec = SubplotSpec(None, row_start, row_end, col_start, col_end)
            ax = _orig_add_subplot(self, spec)
        else:
            raise TypeError(
                f"add_subplot() takes 1 or 3 positional arguments but {len(args)} were given"
            )
        # matplotlib: 余下关键字作为 Axes 属性，经 ax.set(**kwargs) 应用
        if kwargs:
            ax.set(**kwargs)
        return ax

    _rs.Figure.add_subplot = _add_subplot

    def _fig_colorbar(self, mappable=None, cax=None, ax=None, use_gridspec=True, **kwargs):
        """在目标子图上启用颜色条。优先用显式 ax，其次用 mappable 记录的 Axes，最后
        回退到当前 Axes。支持 location/orientation/shrink/aspect/pad/fraction/label/
        extend/ticks/format 等参数（见 `_apply_colorbar`）。"""
        target = ax
        if target is None and mappable is not None:
            target = getattr(mappable, 'axes', None)
        if target is None:
            target = _get_axes()
        _apply_colorbar(target, kwargs)
        return None

    _rs.Figure.colorbar = _fig_colorbar


class _SecondaryAxis:
    """secondary_xaxis / secondary_yaxis 返回的副轴句柄。

    副轴本身不新建坐标系，仅由 Rust 后端在主轴对侧按变换后的刻度绘制刻度线/刻度值。
    此对象持有父 Axes 引用与轴向 ('x'/'y')，让 set_xlabel/set_ylabel 把轴标签回写到
    Rust；其余未实现的链式调用（set_xticks / tick_params 等）由 __getattr__ 吸收为空操作。
    """

    def __init__(self, parent=None, which='x'):
        self._parent = parent
        self._which = which

    def _set_label(self, label):
        if self._parent is not None:
            self._parent.set_secondary_label(self._which, _render_mathtext(str(label)))
        return None

    def set_xlabel(self, label, *args, **kwargs):
        return self._set_label(label)

    def set_ylabel(self, label, *args, **kwargs):
        return self._set_label(label)

    def __getattr__(self, name):
        def _noop(*args, **kwargs):
            return None
        return _noop


class _ScalarMappable:
    """colorbar 所需的可映射句柄（近似）。持有绘制所在的 Axes 引用，
    供 Figure.colorbar 在未显式传 ax 时定位目标子图。"""

    def __init__(self, ax):
        self.axes = ax


def _norm_vminmax(norm, vmin, vmax):
    """从 matplotlib Normalize/LogNorm 对象取 vmin/vmax（显式 vmin/vmax 优先）。"""
    if norm is not None:
        if vmin is None:
            vmin = getattr(norm, 'vmin', None)
        if vmax is None:
            vmax = getattr(norm, 'vmax', None)
    return vmin, vmax


def _norm_kind(norm):
    """返回归一化类型标记 'linear' / 'log'，供 Rust 侧选择上色与颜色条刻度方式。

    识别 rsplotlib.colors.Normalize/LogNorm（带 _norm_kind 属性）以及字符串
    'log' / 'linear'；其余（含 None）默认线性。
    """
    if norm is None:
        return 'linear'
    if isinstance(norm, str):
        return 'log' if norm.strip().lower() == 'log' else 'linear'
    kind = getattr(norm, '_norm_kind', None)
    return kind if kind in ('linear', 'log') else 'linear'


def _seq_minmax(a):
    """序列/数组的 (min, max)，无法解析时返回 (None, None)。"""
    if a is None:
        return None, None
    try:
        if hasattr(a, 'min') and hasattr(a, 'max'):
            return float(a.min()), float(a.max())
    except (TypeError, ValueError):
        pass
    lst = _to_list(a)
    if isinstance(lst, (list, tuple)) and lst:
        try:
            nums = [float(v) for v in lst]
            return min(nums), max(nums)
        except (TypeError, ValueError):
            return None, None
    return None, None


def _patch_axes():
    """为 Rust Axes 类添加 Python 级别的 API 兼容补丁。"""
    from . import rsplotlib as _rs

    # secondary_xaxis / secondary_yaxis: 在主轴对侧绘制变换后的刻度（见 _SecondaryAxis）。
    # functions 为 (forward, inverse) 元组或单个可调用；forward 把主轴数据映射到副轴刻度值。
    def _parse_secondary_functions(functions):
        forward = None
        inverse = None
        if isinstance(functions, (tuple, list)):
            if len(functions) >= 1:
                forward = functions[0]
            if len(functions) >= 2:
                inverse = functions[1]
        elif callable(functions):
            forward = functions
        if not callable(forward):
            def forward(v):
                return v
        if not callable(inverse):
            inverse = None
        return forward, inverse

    def _secondary_xaxis(self, location=None, functions=None, *args, **kwargs):
        forward, inverse = _parse_secondary_functions(functions)
        loc = location if isinstance(location, str) else 'top'
        self.register_secondary_axis('x', loc, forward, inverse)
        return _SecondaryAxis(self, 'x')

    def _secondary_yaxis(self, location=None, functions=None, *args, **kwargs):
        forward, inverse = _parse_secondary_functions(functions)
        loc = location if isinstance(location, str) else 'right'
        self.register_secondary_axis('y', loc, forward, inverse)
        return _SecondaryAxis(self, 'y')

    _rs.Axes.secondary_xaxis = _secondary_xaxis
    _rs.Axes.secondary_yaxis = _secondary_yaxis

    def _get_projection(self):
        return self.get_projection()

    _rs.Axes.projection = property(_get_projection)

    # plot: 支持单参数 ax.plot(y)、格式字符串 ax.plot(y, 'o')/ax.plot(x, y, 'o:r')
    # 以及一次多条线 ax.plot(x1, y1, fmt1, x2, y2, ...)。
    _orig_plot = _rs.Axes.plot

    def _plot(self, *args, **kwargs):
        _map_aliases(kwargs)
        if isinstance(kwargs.get('label'), str):
            kwargs['label'] = _render_mathtext(kwargs['label'])
        pairs, _ = _parse_plot_args(args, kwargs)
        lines = []
        markevery = kwargs.get('markevery')
        for x, y, fmt in pairs:
            call_kwargs = dict(kwargs)
            if fmt:
                # fmt 解析出的样式作为默认值，不覆盖用户显式传入的关键字参数。
                for key, value in _parse_fmt(fmt).items():
                    call_kwargs.setdefault(key, value)
            # 日期坐标：datetime/date 序列的 x 转为自 1970-01-01 起天数的 float。
            xnum = _maybe_dates_to_num(x)
            if xnum is not None:
                x = xnum
            # 分类坐标：字符串 x/y 映射到 0,1,2,... 位置，字符串作为刻度标签。
            x, x_tick_labels = _categorical(x)
            y, y_tick_labels = _categorical(y)
            # 过滤掉 Rust 层不支持的关键字参数
            unsupported_args = {'markevery', 'alpha'}
            call_kwargs = {k: v for k, v in call_kwargs.items() if k not in unsupported_args}
            # 如果有 markevery，先去掉 marker 只画折线，然后用 scatter 单独绘制标记点
            if markevery is not None and call_kwargs.get('marker'):
                # 保存 marker 相关参数
                marker = call_kwargs.pop('marker')
                markersize = call_kwargs.pop('markersize', 6)
                markerfacecolor = call_kwargs.pop('markerfacecolor', None)
                markeredgecolor = call_kwargs.pop('markeredgecolor', None)
                linecolor = call_kwargs.get('color', None)
                # 画折线（不带 marker）
                lines.append(_orig_plot(self, x, y, **call_kwargs))
                # 用 scatter 绘制每隔 markevery 的标记点
                x_list = x if isinstance(x, list) else list(x)
                y_list = y if isinstance(y, list) else list(y)
                indices = list(range(0, len(x_list), markevery))
                x_sub = [x_list[i] for i in indices]
                y_sub = [y_list[i] for i in indices]
                scatter_kwargs = {'marker': marker, 's': markersize ** 2}
                if markerfacecolor is not None:
                    scatter_kwargs['c'] = markerfacecolor
                elif linecolor is not None:
                    scatter_kwargs['c'] = linecolor
                if markeredgecolor is not None:
                    scatter_kwargs['edgecolor'] = markeredgecolor
                self.scatter(x_sub, y_sub, **scatter_kwargs)
            else:
                # 正常绘制（所有点都标记）
                lines.append(_orig_plot(self, x, y, **call_kwargs))
            if x_tick_labels is not None:
                self.set_xticks(list(range(len(x_tick_labels))), x_tick_labels)
            if y_tick_labels is not None:
                self.set_yticks(list(range(len(y_tick_labels))), y_tick_labels)
        # matplotlib: plot() 返回 Line2D 列表, 支持 `l, = ax.plot(...)` 解包。
        return lines

    _rs.Axes.plot = _plot

    # hlines / vlines: Rust 层已支持批量, 直接转发
    _orig_hlines = _rs.Axes.hlines
    _orig_vlines = _rs.Axes.vlines

    def _hlines(self, y, xmin=None, xmax=None, color=None, linestyle=None, linewidth=None, **kwargs):
        y_arr = _to_list(y) if isinstance(y, (list, tuple)) or hasattr(y, 'tolist') else [y]
        return _orig_hlines(self, y_arr, color, linestyle, linewidth)

    def _vlines(self, x, ymin=None, ymax=None, color=None, linestyle=None, linewidth=None, **kwargs):
        x_arr = _to_list(x) if isinstance(x, (list, tuple)) or hasattr(x, 'tolist') else [x]
        return _orig_vlines(self, x_arr, color, linestyle, linewidth)

    _rs.Axes.hlines = _hlines
    _rs.Axes.vlines = _vlines

    # scatter: 支持 c/s 为数组、数值 c + cmap、RGB(A) 二维行数组
    _orig_scatter = _rs.Axes.scatter

    def _scatter(self, x, y, s=None, c=None, marker=None, label=None, alpha=1.0,
                 edgecolor=None, linewidth=None, **kwargs):
        # matplotlib data= 关键字：x/y 位置参数与 c/s 若为字符串键，从 data 中取值。
        data = kwargs.pop('data', None)
        if data is not None:
            x = _replace_from_data(x, data)
            y = _replace_from_data(y, data)
            c = _replace_from_data(c, data)
            s = _replace_from_data(s, data)
        # 复数名 edgecolors/linewidths 为 matplotlib 主用形式（OO API 以关键字传入），
        # 单数名 edgecolor/linewidth 既是别名、也是模块级 scatter() 路由过来的位置参数。
        edgecolors = kwargs.pop('edgecolors', None)
        linewidths = kwargs.pop('linewidths', None)
        if edgecolor is None:
            edgecolor = kwargs.pop('edgecolor', None)
        else:
            kwargs.pop('edgecolor', None)
        if linewidth is None:
            linewidth = kwargs.pop('linewidth', None)
        else:
            kwargs.pop('linewidth', None)
        edgecolor = _coerce_edgecolor(edgecolors if edgecolors is not None else edgecolor)
        linewidth = _coerce_linewidth(linewidths if linewidths is not None else linewidth)
        use_multi, args, mappable = _normalize_scatter(
            x, y, s, c, marker, label=label, alpha=alpha,
            edgecolor=edgecolor, linewidth=linewidth, kwargs=kwargs)
        if mappable is not None:
            self.set_mappable(*mappable)
        if use_multi:
            self.scatter_multi(*args)
        else:
            _orig_scatter(self, *args)
        from .collections import PathCollection
        return PathCollection(None)

    _rs.Axes.scatter = _scatter

    # imshow: 吸收 matplotlib 的 norm 参数（从 Normalize/LogNorm 取 vmin/vmax 与
    # 归一化类型），并返回可映射句柄供 fig.colorbar 使用。
    _orig_imshow = _rs.Axes.imshow

    def _imshow(self, x, cmap='viridis', aspect='equal', vmin=None, vmax=None,
                alpha=None, origin=None, interpolation=None, norm=None, **kwargs):
        vmin, vmax = _norm_vminmax(norm, vmin, vmax)
        cmap = cmap or 'viridis'
        aspect = aspect or 'equal'
        _orig_imshow(self, x, cmap=cmap, aspect=aspect, vmin=vmin, vmax=vmax,
                     alpha=alpha, origin=origin, interpolation=interpolation,
                     norm=_norm_kind(norm))
        return _ScalarMappable(self)

    _rs.Axes.imshow = _imshow

    _orig_tick_params = _rs.Axes.tick_params

    def _tick_params(self, **kwargs):
        kwargs.pop('which', None)
        return _orig_tick_params(self, **kwargs)
    _rs.Axes.tick_params = _tick_params

    # pcolormesh / contourf: 后端未原生支持，近似为 imshow 上色
    # (origin='lower', aspect='auto' 使色块填满子图框)。返回可映射句柄供 colorbar 使用。
    def _pcolormesh(self, *args, **kwargs):
        cmap = kwargs.pop('cmap', None) or 'viridis'
        vmin = kwargs.pop('vmin', None)
        vmax = kwargs.pop('vmax', None)
        norm = kwargs.pop('norm', None)
        vmin, vmax = _norm_vminmax(norm, vmin, vmax)
        z = args[2] if len(args) >= 3 else args[0]
        self.imshow(z, cmap=cmap, aspect='auto', vmin=vmin, vmax=vmax,
                    origin='lower', norm=norm)
        return _ScalarMappable(self)

    def _contourf(self, *args, **kwargs):
        cmap = kwargs.pop('cmap', None) or 'viridis'
        vmin = kwargs.pop('vmin', None)
        vmax = kwargs.pop('vmax', None)
        norm = kwargs.pop('norm', None)
        vmin, vmax = _norm_vminmax(norm, vmin, vmax)
        levels = kwargs.pop('levels', None)
        if levels is not None:
            lo, hi = _seq_minmax(levels)
            vmin = lo if vmin is None else vmin
            vmax = hi if vmax is None else vmax
        z = args[2] if len(args) >= 3 else args[0]
        self.imshow(z, cmap=cmap, aspect='auto', vmin=vmin, vmax=vmax,
                    origin='lower', norm=norm)
        return _ScalarMappable(self)

    _rs.Axes.pcolormesh = _pcolormesh
    _rs.Axes.contourf = _contourf

    # bar / barh: 支持类别坐标（字符串 x/y 映射到 0,1,2,... 位置，字符串作为刻度标签）。
    _orig_bar = _rs.Axes.bar
    _orig_barh = _rs.Axes.barh

    def _bar(self, x, height, width=0.8, color=None, label=None, **kwargs):
        x = _to_list(x)
        tick_labels = None
        if isinstance(x, (list, tuple)) and any(isinstance(v, str) for v in x):
            tick_labels = [str(v) for v in x]
            x = list(range(len(x)))
        if isinstance(label, str):
            label = _render_mathtext(label)
        result = _orig_bar(self, x, height, width, color, label)
        if tick_labels is not None:
            self.set_xticks(list(range(len(tick_labels))), tick_labels)
        return result

    def _barh(self, y, width, height=0.8, color=None, label=None, **kwargs):
        y = _to_list(y)
        tick_labels = None
        if isinstance(y, (list, tuple)) and any(isinstance(v, str) for v in y):
            tick_labels = [str(v) for v in y]
            y = list(range(len(y)))
        if isinstance(label, str):
            label = _render_mathtext(label)
        result = _orig_barh(self, y, width, height, color, label)
        if tick_labels is not None:
            self.set_yticks(list(range(len(tick_labels))), tick_labels)
        return result

    _rs.Axes.bar = _bar
    _rs.Axes.barh = _barh

    # legend: 支持 legend()、legend(loc=...)、legend(handles, labels)。
    # handles 为 Line2D 句柄时，从其 get_color/get_linestyle/... 取样式，labels 取文本，
    # 组装为显式图例条目（替换自动收集的条目）。
    _orig_legend = _rs.Axes.legend

    def _legend(self, *args, **kwargs):
        loc = kwargs.pop('loc', 'best')
        handles = kwargs.pop('handles', None)
        labels = kwargs.pop('labels', None)
        facecolor, framealpha, edgecolor, fontsize = _legend_frame_kwargs(kwargs)
        if len(args) == 2:
            handles, labels = args[0], args[1]
        elif len(args) == 1:
            if isinstance(args[0], str):
                loc = args[0]
            else:
                labels = args[0]
        if not isinstance(loc, str):
            loc = 'best'
        if handles is not None and labels is not None:
            entries = []
            for h, lbl in zip(handles, _to_list(labels)):
                getc = getattr(h, 'get_color', None)
                color = getc() if getc is not None else None
                getls = getattr(h, 'get_linestyle', None)
                ls = getls() if getls is not None else None
                getlw = getattr(h, 'get_linewidth', None)
                lw = getlw() if getlw is not None else None
                getm = getattr(h, 'get_marker', None)
                marker = getm() if getm is not None else None
                color = color if isinstance(color, str) else 'C0'
                ls = ls if (isinstance(ls, str) and ls.strip()) else '-'
                lw = 1.5 if lw is None else float(lw)
                if not (isinstance(marker, str) and marker.strip() not in ('', 'none')):
                    marker = None
                entries.append((_render_mathtext(str(lbl)), color, ls, marker, lw, 1.0))
            return self.set_legend_entries(
                entries, loc, facecolor, framealpha, edgecolor, fontsize)
        return _orig_legend(self, loc, facecolor, framealpha, edgecolor, fontsize)

    _rs.Axes.legend = _legend

    # set_xlim / set_ylim: 支持元组参数 (left, right)
    _orig_set_xlim = _rs.Axes.set_xlim
    _orig_set_ylim = _rs.Axes.set_ylim

    def _set_xlim(self, *args, **kwargs):
        if len(args) == 1 and isinstance(args[0], (tuple, list)):
            lo, hi = args[0]
            return _orig_set_xlim(self, left=lo, right=hi)
        return _orig_set_xlim(self, *args, **kwargs)

    def _set_ylim(self, *args, **kwargs):
        if len(args) == 1 and isinstance(args[0], (tuple, list)):
            lo, hi = args[0]
            return _orig_set_ylim(self, bottom=lo, top=hi)
        return _orig_set_ylim(self, *args, **kwargs)

    _rs.Axes.set_xlim = _set_xlim
    _rs.Axes.set_ylim = _set_ylim

    # set_xticks / set_yticks: 支持第二个位置参数 labels，且把数组对象归一为 list。
    _orig_set_xticks = _rs.Axes.set_xticks
    _orig_set_yticks = _rs.Axes.set_yticks

    def _set_xticks(self, ticks=None, labels=None, **kwargs):
        ticks = _to_list(ticks) if ticks is not None else None
        labels = [str(x) for x in _to_list(labels)] if labels is not None else None
        return _orig_set_xticks(self, ticks, labels)

    def _set_yticks(self, ticks=None, labels=None, **kwargs):
        ticks = _to_list(ticks) if ticks is not None else None
        labels = [str(x) for x in _to_list(labels)] if labels is not None else None
        return _orig_set_yticks(self, ticks, labels)

    _rs.Axes.set_xticks = _set_xticks
    _rs.Axes.set_yticks = _set_yticks

    # set_xticklabels / set_yticklabels: 归一为字符串 list，吸收 rotation/ha/fontsize 等
    # 未支持的样式 kwargs。须在 set_xticks/set_yticks 固定刻度位置后调用（matplotlib 语义）。
    _orig_set_xticklabels = _rs.Axes.set_xticklabels
    _orig_set_yticklabels = _rs.Axes.set_yticklabels

    def _set_xticklabels(self, labels, **kwargs):
        return _orig_set_xticklabels(self, [str(x) for x in _to_list(labels)])

    def _set_yticklabels(self, labels, **kwargs):
        return _orig_set_yticklabels(self, [str(x) for x in _to_list(labels)])

    _rs.Axes.set_xticklabels = _set_xticklabels
    _rs.Axes.set_yticklabels = _set_yticklabels

    # axis: 支持序列 [xmin,xmax,ymin,ymax] 设定轴限，及 'off'/'on'/'equal' 等字符串。
    _orig_axis = _rs.Axes.axis

    def _axis(self, arg=None, **kwargs):
        if isinstance(arg, (list, tuple)):
            if len(arg) == 4:
                xmin, xmax, ymin, ymax = arg
                self.set_xlim(xmin, xmax)
                self.set_ylim(ymin, ymax)
            return None
        if arg is False:
            arg = 'off'
        elif arg is True:
            arg = 'on'
        if isinstance(arg, str):
            if arg in ('equal', 'scaled', 'image', 'square'):
                setter = getattr(self, 'set_aspect', None)
                if setter is not None:
                    setter('equal')
                return None
            return _orig_axis(self, arg)
        return _orig_axis(self, None)

    _rs.Axes.axis = _axis

    # autoscale(enable=True, axis='both', tight=None): rsplotlib 默认按数据自动计算
    # 坐标范围，启用自动缩放即默认行为；关闭 (enable=False) 当前不支持冻结范围，按空操作处理。
    def _autoscale(self, enable=True, axis='both', tight=None):
        return None

    _rs.Axes.autoscale = _autoscale

    # 文本类方法：把 matplotlib mathtext ($...$) 转成 Unicode 后再下沉到 Rust。
    # OO API (ax.set_xlabel / ax.text ...) 不经过模块级 plt.* 函数，需在此单独接入。
    _orig_set_xlabel = _rs.Axes.set_xlabel
    _orig_set_ylabel = _rs.Axes.set_ylabel
    _orig_set_title = _rs.Axes.set_title

    def _set_xlabel(self, text, *args, **kwargs):
        return _orig_set_xlabel(self, _render_mathtext(text), *args, **kwargs)

    def _set_ylabel(self, text, *args, **kwargs):
        return _orig_set_ylabel(self, _render_mathtext(text), *args, **kwargs)

    def _set_title(self, text, *args, **kwargs):
        return _orig_set_title(self, _render_mathtext(text), *args, **kwargs)

    _rs.Axes.set_xlabel = _set_xlabel
    _rs.Axes.set_ylabel = _set_ylabel
    _rs.Axes.set_title = _set_title

    _orig_text = _rs.Axes.text

    def _text(self, x, y, s, fontsize=None, color=None, c=None, family=None, rotation=None, horizontalalignment='center', verticalalignment='center', transform=None, bbox=None, clip_on=None, alpha=None, weight=None, dx=None, dy=None, **kwargs):
        if not isinstance(s, str):
            s = str(s)
        if fontsize is None:
            fontsize = kwargs.get('size', None)
        if hasattr(x, 'item'):
            x = x.item()
        elif not isinstance(x, (int, float)):
            x = float(x)
        if hasattr(y, 'item'):
            y = y.item()
        elif not isinstance(y, (int, float)):
            y = float(y)
        if rotation is None:
            rotation = 0.0
        s_rendered = _render_mathtext(s)
        return _orig_text(self, x, y, s_rendered, fontsize, color, c, family, horizontalalignment, verticalalignment, rotation, dx, dy, bbox)

    _rs.Axes.text = _text

    _orig_annotate = _rs.Axes.annotate

    def _annotate(self, text, xy, xytext=None, fontsize=None,
                  color="black", arrowprops=None, xycoords='data',
                  textcoords=None, ha=None, family=None, **kwargs):
        # 支持坐标系 (xycoords/textcoords)、水平对齐 (ha/horizontalalignment)、
        # 字体族 (family/fontfamily)；其余未支持参数 (如 va) 被 **kwargs 吸收后丢弃。
        if isinstance(text, str):
            text = _render_mathtext(text)
        # 未显式指定 fontsize 时，默认字号随其余默认字号一并放大 DEFAULT_FONT_SCALE 倍；
        # 用户显式传入 fontsize 则保持原值不放大。
        if fontsize is None:
            fontsize = _DEFAULT_ANNOTATE_FONTSIZE
        if ha is None:
            ha = kwargs.get('horizontalalignment', 'center')
        if family is None:
            family = kwargs.get('fontfamily', None)
        # xycoords 可能是 get_xaxis_transform / get_yaxis_transform 返回的标记字符串；
        # 若传入的是其它非字符串的 transform 对象则回退到 'data'。
        if not isinstance(xycoords, str):
            xycoords = 'data'
        if textcoords is not None and not isinstance(textcoords, str):
            textcoords = None
        # 处理 xy 中的 0-d 数组对象
        if isinstance(xy, (list, tuple)) and len(xy) >= 2:
            def _to_float(v):
                if hasattr(v, 'ndim') and v.ndim == 0:
                    return float(v.item()) if hasattr(v, 'item') else float(v)
                return float(v)
            xy = (_to_float(xy[0]), _to_float(xy[1]))
        return _orig_annotate(self, text, xy, xytext, fontsize, color,
                              arrowprops, xycoords, textcoords, ha, family)

    _rs.Axes.annotate = _annotate

    class _Table:
        def __init__(self, ax, cell_text, col_widths, row_labels, col_labels, row_colors, loc):
            self.ax = ax
            self._cell_text = cell_text
            self._col_widths = col_widths
            self._row_labels = row_labels
            self._col_labels = col_labels
            self._row_colors = row_colors
            self._loc = loc
            self._fontsize = 10.0

        def auto_set_font_size(self, auto):
            pass

        def set_fontsize(self, size):
            self._fontsize = size

        def scale(self, xscale, yscale):
            pass

    def _table(self, cellText=None, colWidths=None, rowLabels=None, colLabels=None,
               rowColours=None, loc='bottom', **kwargs):
        cell_text = cellText or []
        col_widths = colWidths or []
        row_labels = rowLabels or []
        col_labels = colLabels or []
        row_colors = rowColours or []

        table_obj = _Table(self, cell_text, col_widths, row_labels, col_labels, row_colors, loc)
        fontsize = kwargs.get('fontsize', 10.0)
        table_obj.set_fontsize(fontsize)

        _orig_table(self, cell_text, col_widths, row_labels, col_labels, row_colors, loc)

        return table_obj

    _orig_table = _rs.Axes.table
    _rs.Axes.table = _table

    # set(**kwargs): matplotlib 语义, 每个 key 映射到 set_<key>(value)
    def _ax_set(self, **kwargs):
        for key, value in kwargs.items():
            setter = getattr(self, 'set_' + key, None)
            if setter is None:
                continue
            # 数组对象转 list, 供 Rust 侧 Vec<f64> 提取; tuple 保留给 set_xlim 处理
            if hasattr(value, 'tolist'):
                value = value.tolist()
            setter(value)
        return None

    _rs.Axes.set = _ax_set

    class _Transform:
        def transform(self, coords):
            return coords

        def inverted(self):
            return self

    _rs.Axes.transData = _Transform()

    def _to_color_string(color):
        """Convert a color value (string, list, tuple) to a CSS color string."""
        if isinstance(color, str):
            return color
        if isinstance(color, (list, tuple)):
            n = len(color)
            if n == 3:
                r = int(max(0, min(1, color[0])) * 255)
                g = int(max(0, min(1, color[1])) * 255)
                b = int(max(0, min(1, color[2])) * 255)
                return f'rgb({r},{g},{b})'
            if n == 4:
                r = int(max(0, min(1, color[0])) * 255)
                g = int(max(0, min(1, color[1])) * 255)
                b = int(max(0, min(1, color[2])) * 255)
                a = float(color[3])
                return f'rgba({r},{g},{b},{a})'
        return 'black'

    def _add_collection(self, collection, autolim=True):
        from rsplotlib.collections import LineCollection
        if isinstance(collection, LineCollection):
            segments = collection.segments
            if segments is None:
                return
            color = _to_color_string(collection.colors) if collection.colors else 'black'
            linewidth = collection.linewidths if collection.linewidths else 1.0
            linestyle = collection.linestyle if collection.linestyle else '-'
            alpha = collection.alpha if collection.alpha else None
            
            if hasattr(segments, 'tolist'):
                segments = segments.tolist()
            
            for i, seg in enumerate(segments):
                if len(seg) >= 2:
                    x = [float(p[0]) for p in seg]
                    y = [float(p[1]) for p in seg]
                    try:
                        self.plot(x, y, color=color, linewidth=linewidth, linestyle=linestyle, alpha=alpha)
                    except Exception:
                        pass

    _rs.Axes.add_collection = _add_collection

    def _update_datalim(self, xydata, update_datalim=True):
        pass

    _rs.Axes.update_datalim = _update_datalim

    def _autoscale_view(self, tight=None, scalex=True, scaley=True):
        pass
    _rs.Axes.autoscale_view = _autoscale_view

    def _add_artist(self, artist):
        # Edge label text rendering from CurvedArrowTextBase
        is_edge_label = hasattr(artist, '_update_text_pos_angle') and hasattr(artist, 'arrow')
        if is_edge_label:
            text_str = artist.text if hasattr(artist, 'text') else ''
            family = getattr(artist, 'family', None)
            fontsize = artist.fontsize
            color = artist.color
            ha = artist.horizontalalignment
            va = artist.verticalalignment
            x, y = artist.x, artist.y

            lines = text_str.split('\n')
            if len(lines) <= 1:
                self.text(x, y, text_str, fontsize=fontsize, color=color, rotation=0,
                          family=family, horizontalalignment=ha, verticalalignment=va,
                          bbox=dict(facecolor='white', edgecolor='none', pad=1, alpha=1.0))
                return

            # 多行文本：全部水平显示（rotation=0），上下排列
            # 编号在上，阻抗在下
            line_height = fontsize * 0.002
            
            n = len(lines)
            
            # 绘制文字（使用 bbox 参数添加白色背景）
            for i, line in enumerate(lines):
                # idx 越小的行越靠上（y_offset为正 = 向上偏移，因为数据坐标y越大越靠上）
                y_offset = - (i - (n - 1) / 2.0) * line_height
                self.text(x, y + y_offset, line, fontsize=fontsize, color=color, rotation=0,
                          family=family, horizontalalignment='center', verticalalignment='center',
                          bbox=dict(facecolor='white', edgecolor='none', pad=0.5, alpha=1.0))
    _rs.Axes.add_artist = _add_artist

    def _add_patch(self, patch):
        from rsplotlib.patches import FancyArrowPatch, _Path
        if isinstance(patch, FancyArrowPatch):
            posA = patch.posA
            posB = patch.posB
            color = _to_color_string(patch.color) if patch.color else 'black'
            linewidth = patch.linewidth if patch.linewidth else 1.0
            
            # Apply shrinkA/shrinkB to adjust endpoints
            shrinkA = patch.shrinkA if patch.shrinkA else 0.0
            shrinkB = patch.shrinkB if patch.shrinkB else 0.0
            
            x1, y1 = posA
            x2, y2 = posB
            
            # Calculate direction vector
            dx = x2 - x1
            dy = y2 - y1
            length = math.sqrt(dx*dx + dy*dy)
            
            # 标记是否需要在标签位置断开线
            need_break = False
            break_x1, break_y1 = x1, y1
            break_x2, break_y2 = x2, y2
            
            if length > 0:
                # Apply shrink
                if shrinkA > 0 or shrinkB > 0:
                    ux, uy = dx / length, dy / length
                    x1 += ux * shrinkA
                    y1 += uy * shrinkA
                    x2 -= ux * shrinkB
                    y2 -= uy * shrinkB
                
                # 如果边有标签，在标签位置断开线
                # 标签位于边的中间位置，留出约 15% 的边长度作为标签区域
                if hasattr(patch, 'arrow') or hasattr(patch, '_update_text_pos_angle'):
                    need_break = True
                    label_region = length * 0.15
                    ux, uy = dx / length, dy / length
                    # 第一段终点 = 起点 + (总长度 - 标签区域) / 2
                    break_x1 = x1 + ux * (length - label_region) / 2
                    break_y1 = y1 + uy * (length - label_region) / 2
                    # 第二段起点 = 终点 - (总长度 - 标签区域) / 2
                    break_x2 = x2 - ux * (length - label_region) / 2
                    break_y2 = y2 - uy * (length - label_region) / 2
            
            # Use connectionstyle to get the curve path
            conn = patch.get_connectionstyle()
            if conn is not None:
                path = conn((x1, y1), (x2, y2))
                if isinstance(path, _Path) and len(path.vertices) >= 3:
                    if need_break:
                        # 断开绘制：第一段从起点到标签区域起点
                        path1 = conn((x1, y1), (break_x1, break_y1))
                        if isinstance(path1, _Path) and len(path1.vertices) >= 3:
                            vertices = path1.vertices
                            xs, ys = [], []
                            for i in range(21):
                                t = i / 20
                                tt = 1 - t
                                px = tt*tt * vertices[0][0] + 2*tt*t * vertices[1][0] + t*t * vertices[2][0]
                                py = tt*tt * vertices[0][1] + 2*tt*t * vertices[1][1] + t*t * vertices[2][1]
                                xs.append(px)
                                ys.append(py)
                            self.plot(xs, ys, color=color, linewidth=linewidth)
                        else:
                            self.plot([x1, break_x1], [y1, break_y1], color=color, linewidth=linewidth)
                        # 第二段从标签区域终点到终点
                        path2 = conn((break_x2, break_y2), (x2, y2))
                        if isinstance(path2, _Path) and len(path2.vertices) >= 3:
                            vertices = path2.vertices
                            xs, ys = [], []
                            for i in range(21):
                                t = i / 20
                                tt = 1 - t
                                px = tt*tt * vertices[0][0] + 2*tt*t * vertices[1][0] + t*t * vertices[2][0]
                                py = tt*tt * vertices[0][1] + 2*tt*t * vertices[1][1] + t*t * vertices[2][1]
                                xs.append(px)
                                ys.append(py)
                            self.plot(xs, ys, color=color, linewidth=linewidth)
                        else:
                            self.plot([break_x2, x2], [break_y2, y2], color=color, linewidth=linewidth)
                    else:
                        vertices = path.vertices
                        xs, ys = [], []
                        for i in range(21):
                            t = i / 20
                            tt = 1 - t
                            px = tt*tt * vertices[0][0] + 2*tt*t * vertices[1][0] + t*t * vertices[2][0]
                            py = tt*tt * vertices[0][1] + 2*tt*t * vertices[1][1] + t*t * vertices[2][1]
                            xs.append(px)
                            ys.append(py)
                        self.plot(xs, ys, color=color, linewidth=linewidth)
                else:
                    if need_break:
                        self.plot([x1, break_x1], [y1, break_y1], color=color, linewidth=linewidth)
                        self.plot([break_x2, x2], [break_y2, y2], color=color, linewidth=linewidth)
                    else:
                        self.plot([x1, x2], [y1, y2], color=color, linewidth=linewidth)
            else:
                self.plot([x1, x2], [y1, y2], color=color, linewidth=linewidth)
            
            arrowstyle = patch.arrowstyle
            if arrowstyle and arrowstyle != '-':
                self.annotate('', xy=(x2, y2), xytext=(x1, y1), arrowprops=dict(color=color, arrowstyle=arrowstyle, linewidth=linewidth))

    _rs.Axes.add_patch = _add_patch


_patch_figure_add_subplot()
_patch_axes()
_patch_axes_get_gridspec()
_patch_axes_remove()


style = _style_module.style
