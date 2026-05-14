import sys
from PySide6.QtWidgets import QApplication, QWidget, QVBoxLayout, QPushButton, QHBoxLayout
from PySide6.QtGui import QImage, QPixmap, QIcon
from PySide6.QtCore import QSize, Qt
from AppKit import (NSImage, NSImageSymbolConfiguration, NSBitmapImageRep,
                    NSCalibratedRGBColorSpace, NSGraphicsContext)

def get_sf_qicon(symbol_name, size=24, weight=1):
    base_image = NSImage.imageWithSystemSymbolName_accessibilityDescription_(symbol_name, None)
    if not base_image:
        return QIcon()

    # 1. TRICK: Request a HUGE point size (200) with UltraLight (1) weight
    # and Small (1) scale. This forces the thinnest possible vector lines.
    config = NSImageSymbolConfiguration.configurationWithPointSize_weight_scale_(200, weight, 1)
    ns_image = base_image.imageWithSymbolConfiguration_(config)

    # 2. Calculate the shape
    native_size = ns_image.size()
    aspect_ratio = native_size.width / native_size.height

    # 3. Target drawing size
    draw_width = size * aspect_ratio
    draw_height = size
    scale = 2.0
    pixel_w = int(draw_width * scale)
    pixel_h = int(draw_height * scale)

    # 4. Create the canvas
    bitmap = NSBitmapImageRep.alloc().initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel_(
        None, pixel_w, pixel_h, 8, 4, True, False, NSCalibratedRGBColorSpace, 0, 0
    )
    bitmap.setSize_((draw_width, draw_height))

    # 5. Draw
    context = NSGraphicsContext.graphicsContextWithBitmapImageRep_(bitmap)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.setCurrentContext_(context)
    # This shrinks the 200pt 'Thin' lines into your small 24pt box
    ns_image.drawInRect_(((0, 0), (draw_width, draw_height)))
    NSGraphicsContext.restoreGraphicsState()

    # 6. Convert
    png_data = bitmap.representationUsingType_properties_(4, None)
    q_image = QImage.fromData(png_data.bytes().tobytes())
    q_image.setDevicePixelRatio(scale)

    return QIcon(QPixmap.fromImage(q_image))

class MyWindow(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Thin Symbols")
        layout = QVBoxLayout()

        # Use weight=1 (UltraLight) and icons WITHOUT ".fill"
        self.btn = QPushButton(" Ultra Thin")
        self.btn.setIcon(get_sf_qicon("house", size=24, weight=1))
        self.btn.setIconSize(QSize(40, 24))

        layout.addWidget(self.btn)
        self.setLayout(layout)

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MyWindow()
    window.show()
    sys.exit(app.exec())
