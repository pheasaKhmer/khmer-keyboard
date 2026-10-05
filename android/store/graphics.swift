// Draws the Play Store icon (icon-512.png) and feature graphic (feature-graphic.png) into the
// directory given as the only argument:
//
//     swift android/store/graphics.swift android/store
//
// It uses AppKit, so Khmer is shaped by the system's text engine. The screenshots were
// taken on an Android 17 emulator and cropped to 1080 × 1810, within Play's 2:1 limit.

import AppKit

let navy = NSColor(srgbRed: 0x1F / 255, green: 0x3A / 255, blue: 0x68 / 255, alpha: 1)
let out = CommandLine.arguments[1]

func render(_ width: Int, _ height: Int, _ name: String, _ draw: (CGContext) -> Void) {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let cg = NSGraphicsContext.current!.cgContext
    // Draw with the origin at the top left, like the vector icon.
    cg.translateBy(x: 0, y: CGFloat(height))
    cg.scaleBy(x: 1, y: -1)
    draw(cg)
    NSGraphicsContext.restoreGraphicsState()
    let png = rep.representation(using: .png, properties: [:])!
    try! png.write(to: URL(fileURLWithPath: "\(out)/\(name)"))
}

/// The launcher icon's keyboard, in its 108-unit viewport, scaled and moved.
func keyboard(_ cg: CGContext, scale: CGFloat, x: CGFloat, y: CGFloat) {
    cg.saveGState()
    cg.translateBy(x: x, y: y)
    cg.scaleBy(x: scale, y: scale)
    cg.setFillColor(NSColor.white.cgColor)
    cg.addPath(CGPath(roundedRect: CGRect(x: 30, y: 40, width: 48, height: 30), cornerWidth: 4, cornerHeight: 4, transform: nil))
    cg.fillPath()
    cg.setFillColor(navy.cgColor)
    for row in [45.0, 52.0] {
        for column in [35.0, 43.0, 51.0, 59.0, 67.0] {
            cg.fill(CGRect(x: column, y: row, width: 6, height: 5))
        }
    }
    cg.fill(CGRect(x: 43, y: 60, width: 22, height: 5))
    cg.restoreGState()
}

func text(_ string: String, font: NSFont, color: NSColor, at point: CGPoint, flipped height: CGFloat) {
    // Text is drawn in AppKit's bottom-left coordinates; undo the flip for it.
    let attributes: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
    let line = NSAttributedString(string: string, attributes: attributes)
    NSGraphicsContext.current!.cgContext.saveGState()
    NSGraphicsContext.current!.cgContext.scaleBy(x: 1, y: -1)
    NSGraphicsContext.current!.cgContext.translateBy(x: 0, y: -height)
    line.draw(at: NSPoint(x: point.x, y: height - point.y))
    NSGraphicsContext.current!.cgContext.restoreGState()
}

render(512, 512, "icon-512.png") { cg in
    cg.setFillColor(navy.cgColor)
    cg.fill(CGRect(x: 0, y: 0, width: 512, height: 512))
    // Larger than in the launcher, where the system adds its own margin.
    keyboard(cg, scale: 6.6, x: 256 - 54 * 6.6, y: 256 - 55 * 6.6)
}

render(1024, 500, "feature-graphic.png") { cg in
    cg.setFillColor(navy.cgColor)
    cg.fill(CGRect(x: 0, y: 0, width: 1024, height: 500))
    // The keyboard, centred vertically on the left.
    keyboard(cg, scale: 5.2, x: -110, y: -46)
    let latin = NSFont.systemFont(ofSize: 64, weight: .semibold)
    let khmer = NSFont(name: "Khmer Sangam MN", size: 44)!
    let small = NSFont.systemFont(ofSize: 30, weight: .regular)
    let pale = NSColor(srgbRed: 0.78, green: 0.84, blue: 0.95, alpha: 1)
    text("Khmer Keyboard", font: latin, color: .white, at: CGPoint(x: 400, y: 200), flipped: 500)
    text("sok sabay te", font: NSFont.monospacedSystemFont(ofSize: 34, weight: .regular), color: pale, at: CGPoint(x: 404, y: 285), flipped: 500)
    text("→", font: NSFont.systemFont(ofSize: 34), color: .white, at: CGPoint(x: 668, y: 285), flipped: 500)
    text("សុខសប្បាយទេ", font: khmer, color: .white, at: CGPoint(x: 718, y: 292), flipped: 500)
    text("Type Khmer the way you chat", font: small, color: pale, at: CGPoint(x: 404, y: 370), flipped: 500)
}
