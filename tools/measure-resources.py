"""Run after: cargo build -p yue-resource-probe --release --features bundled-fonts.

Inspect complete resource bytes in PE files, independently of total exe size.
This measures linker retention, not runtime SVG parsing or GPU performance.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "crates/yue-component-icons/assets/tabler"
ICONS = sorted(ASSETS.glob("*/*.svg"))
FONTS = sorted((ROOT / "crates/yue-component/assets/fonts").glob("*.ttf"))
report = {
    "profile": "release (Cargo default, x86_64-pc-windows-msvc)",
    "catalogIcons": len(ICONS),
    "svgSourceBytes": sum(path.stat().st_size for path in ICONS),
    "fontSourceBytes": sum(path.stat().st_size for path in FONTS),
    "binaries": {},
}
for name, expected in [("resources-empty", 0), ("resources-one", 1),
                       ("resources-all", len(ICONS)), ("resources-fonts", 0)]:
    data = (ROOT / f"target/release/{name}.exe").read_bytes()
    included = [p.relative_to(ASSETS).as_posix() for p in ICONS if p.read_bytes() in data]
    fonts = sum(path.read_bytes() in data for path in FONTS)
    assert len(included) == expected, (name, len(included), expected)
    assert fonts == (len(FONTS) if name == "resources-fonts" else 0), (name, fonts)
    if name == "resources-one":
        assert included == ["outline/search.svg"], included
    report["binaries"][name] = {
        "bytes": len(data), "embeddedSvgCount": len(included),
        "containsSearch": "outline/search.svg" in included, "embeddedFontCount": fonts,
    }
for name, expected_fonts in [("minimal-system", 0), ("minimal-bundled", len(FONTS))]:
    path = ROOT / f"target/resource-measurements/{name}.exe"
    if not path.exists():
        continue
    data = path.read_bytes()
    fonts = sum(font.read_bytes() in data for font in FONTS)
    assert fonts == expected_fonts, (name, fonts, expected_fonts)
    report["binaries"][name] = {"bytes": len(data), "embeddedFontCount": fonts}
output = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
(ROOT / "docs/resource-measurements.json").write_text(output, encoding="utf-8")
print(output)
