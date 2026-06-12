"""rsplotlib._font_resolver - 字体族名 → 字体文件路径解析

将 matplotlib 风格的无衬线字体族名（如 "Arial Unicode MS"、"Helvetica Neue"）
映射到本地字体文件路径。找不到时返回 None，由调用方决定回退到默认字体。
"""
import os
from typing import Optional, List
import sys


# ====== 字体族名 → 候选文件路径映射（按平台分别维护）======

_FONT_NAME_TO_PATHS = {
    # macOS
    "Arial Unicode MS": [
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ],
    "Arial": [
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ],
    "Helvetica": [
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/HelveticaNeue.ttc",
    ],
    "Helvetica Neue": [
        "/System/Library/Fonts/HelveticaNeue.ttc",
    ],
    "PingFang SC": [
        "/System/Library/Fonts/PingFang.ttc",
    ],
    "Heiti SC": [
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
    ],
    "Hiragino Sans GB": [
        "/System/Library/Fonts/Hiragino Sans GB W3.otf",
        "/System/Library/Fonts/Hiragino Sans GB W6.otf",
    ],
    # Linux
    "DejaVu Sans": [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    ],
    "Liberation Sans": [
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ],
    "Noto Sans CJK SC": [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ],
    "WenQuanYi Micro Hei": [
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    ],
    # Windows
    "Microsoft YaHei": [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/msyhbd.ttc",
    ],
    "SimHei": [
        "C:/Windows/Fonts/simhei.ttf",
    ],
    "SimSun": [
        "C:/Windows/Fonts/simsun.ttc",
    ],
}


# 跨平台按系统名归一化的额外兜底字体路径

def _system_fallback_paths() -> List[str]:
    """按当前操作系统返回一组"通用全功能字体"候选路径"""
    system = sys.platform
    if system == "darwin":
        return [
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ]
    elif system == "linux":
        return [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        ]
    elif system == "win32":
        return [
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/msyh.ttf",
            "C:/Windows/Fonts/msyhbd.ttc",
        ]
    elif system == "cygwin":
        return [
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/arial.ttf",
        ]
    return []


def resolve_font_path(family: str) -> Optional[str]:
    """根据字体族名查找本地的字体文件路径。

    找不到时返回 None。
    """
    if not family:
        return None
    # 精确匹配
    candidates = _FONT_NAME_TO_PATHS.get(family, [])
    for path in candidates:
        if os.path.isfile(path):
            return path
    # 大小写不敏感匹配
    lower_map = {k.lower(): v for k, v in _FONT_NAME_TO_PATHS.items()}
    candidates = lower_map.get(family.lower(), [])
    for path in candidates:
        if os.path.isfile(path):
            return path
    # 平台回退
    for path in _system_fallback_paths():
        if os.path.isfile(path):
            return path
    return None


def apply_rcparams_font() -> Optional[str]:
    """读取 rcParams["font.sans-serif"]，把第一个能解析到本地文件的字体注册到 plotters。

    返回实际注册的字体文件路径，如果没找到任何字体则返回 None。
    """
    try:
        # 延迟导入避免循环依赖
        from . import rsplotlib as _rsplotlib
        from .pylab import mpl
    except Exception:
        return None

    sans_serif = mpl.rcParams.get("font.sans-serif")
    if not sans_serif:
        return None

    if isinstance(sans_serif, str):
        candidates = [sans_serif]
    else:
        try:
            candidates = list(sans_serif)
        except TypeError:
            candidates = [str(sans_serif)]

    # "sans-serif" 关键字跳过：让 plotters 使用内部默认
    candidates = [c for c in candidates if c and c.lower() != "sans-serif"]

    for family in candidates:
        path = resolve_font_path(family)
        if path is None:
            # 可能是直接的字体文件路径
            if os.path.isfile(family):
                path = family
        if path is not None:
            try:
                _rsplotlib.register_sans_serif_font(path)
                return path
            except Exception:
                continue
    return None
