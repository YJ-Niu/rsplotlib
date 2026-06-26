"""rsplotlib._font_resolver - 字体族名 → 字体文件路径解析

将 matplotlib 风格的无衬线字体族名（如 "Arial Unicode MS"、"Helvetica Neue"）
映射到本地字体文件路径。找不到时返回 None，由调用方决定回退到默认字体。

底层实现: Rust font_resolver
"""

from .. import rsplotlib as _rs


def resolve_font_path(family: str) -> str or None:
    """根据字体族名查找本地的字体文件路径。

    底层实现: Rust resolve_font_path

    Args:
        family: 字体族名

    Returns:
        字体文件路径，找不到时返回 None
    """
    return _rs.resolve_font_path(family)


def apply_rcparams_font() -> str or None:
    """读取 rcParams["font.sans-serif"]，把第一个能解析到本地文件的字体注册到 plotters。

    底层实现: Rust apply_rcparams_font

    Returns:
        实际注册的字体文件路径，如果没找到任何字体则返回 None
    """
    return _rs.apply_rcparams_font()
