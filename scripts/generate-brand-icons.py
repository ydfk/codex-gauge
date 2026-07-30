#!/usr/bin/env python3

from pathlib import Path
import subprocess
import tempfile

from PIL import Image


ROOT = Path(__file__).resolve().parent.parent
VECTOR_SOURCE = ROOT / "assets/brand/codex-gauge-logo.svg"
RASTER_SOURCE = ROOT / "assets/brand/codex-gauge-logo.png"
TAURI_ICONS = ROOT / "src-tauri/icons"
APP_ICONSET = ROOT / "native-macos/CodexGaugeNative/Assets.xcassets/AppIcon.appiconset"


def resized(image: Image.Image, size: int) -> Image.Image:
    return image.resize((size, size), Image.Resampling.LANCZOS)


def write_png(image: Image.Image, size: int, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    resized(image, size).save(destination, optimize=True)


def main() -> None:
    subprocess.run(
        [
            "sips",
            "-s",
            "format",
            "png",
            "-z",
            "1024",
            "1024",
            str(VECTOR_SOURCE),
            "--out",
            str(RASTER_SOURCE),
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    source = Image.open(RASTER_SOURCE).convert("RGBA")
    if source.width != source.height:
        raise ValueError("Logo 母版必须是正方形")

    write_png(source, 512, TAURI_ICONS / "icon.png")
    write_png(source, 256, TAURI_ICONS / "tray.png")
    source.save(
        TAURI_ICONS / "icon.ico",
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    app_sizes = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }
    for filename, size in app_sizes.items():
        write_png(source, size, APP_ICONSET / filename)

    with tempfile.TemporaryDirectory(prefix="codex-gauge-icon-") as temporary:
        iconset = Path(temporary) / "CodexGauge.iconset"
        iconset.mkdir()
        for filename, size in app_sizes.items():
            write_png(source, size, iconset / filename)
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(TAURI_ICONS / "icon.icns")],
            check=True,
        )

    print("已生成 macOS、Windows 与托盘图标")


if __name__ == "__main__":
    main()
