import AppKit
import Combine
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let model = AppModel()
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let popover = NSPopover()
    private var modelObserver: AnyCancellable?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.accessory)
        configurePopover()
        configureStatusItem()
        observeModel()
        updateStatusItem()

        Task {
            await model.startIfNeeded()
        }
    }

    private func configurePopover() {
        popover.behavior = .transient
        popover.animates = false
        popover.contentSize = NSSize(width: 376, height: 510)
        popover.contentViewController = NSHostingController(rootView: MenuBarPanel(model: model))
    }

    private func configureStatusItem() {
        guard let button = statusItem.button else { return }
        button.target = self
        button.action = #selector(togglePopover)
        button.sendAction(on: [.leftMouseUp])
        button.imagePosition = .imageLeading
        button.imageScaling = .scaleProportionallyDown
        button.setAccessibilityLabel("Codex Gauge")
    }

    private func observeModel() {
        modelObserver = model.objectWillChange.sink { [weak self] _ in
            DispatchQueue.main.async {
                self?.updateStatusItem()
            }
        }
    }

    private func updateStatusItem() {
        guard let button = statusItem.button else { return }
        button.image = menuBarIcon()
        button.attributedTitle = statusTitle()
        button.toolTip = model.statusText
        button.setAccessibilityValue(model.menuBarTitle.isEmpty ? model.statusText : model.menuBarTitle)
    }

    private func menuBarIcon() -> NSImage? {
        guard let source = NSImage(named: "MenuBarIcon"),
              let image = source.copy() as? NSImage else {
            return nil
        }
        image.size = NSSize(width: 18, height: 18)
        image.isTemplate = true
        return image
    }

    private func statusTitle() -> NSAttributedString {
        let result = NSMutableAttributedString()
        for segment in statusSegments() {
            result.append(
                NSAttributedString(
                    string: segment.text,
                    attributes: titleAttributes(color: segment.color)
                )
            )
        }
        return result
    }

    private func statusSegments() -> [StatusTitleSegment] {
        guard model.config.menuBarDisplay != .iconOnly else { return [] }
        let snapshot = model.snapshot

        if snapshot?.primaryWindowUnlimited == true {
            guard let weekly = snapshot?.secondaryWindow?.remainingPercent else {
                return [StatusTitleSegment(text: " 无限", color: .labelColor)]
            }
            return usageSegments(prefix: " 7d ", remaining: weekly)
        }

        var segments = usageSegments(prefix: " 5h ", remaining: snapshot?.primaryWindow?.remainingPercent)
        if model.config.menuBarDisplay == .fiveAndSeven {
            segments.append(StatusTitleSegment(text: " · 7d ", color: .labelColor))
            segments.append(contentsOf: percentageSegments(snapshot?.secondaryWindow?.remainingPercent))
        }
        return segments
    }

    private func usageSegments(prefix: String, remaining: Double?) -> [StatusTitleSegment] {
        [StatusTitleSegment(text: prefix, color: .labelColor)] + percentageSegments(remaining)
    }

    private func percentageSegments(_ remaining: Double?) -> [StatusTitleSegment] {
        let text = remaining.map { "\(Int($0.rounded()))%" } ?? "--"
        return [StatusTitleSegment(text: text, color: .labelColor)]
    }

    private func titleAttributes(color: NSColor) -> [NSAttributedString.Key: Any] {
        let shadow = NSShadow()
        shadow.shadowColor = NSColor.black.withAlphaComponent(0.35)
        shadow.shadowBlurRadius = 0.75
        shadow.shadowOffset = NSSize(width: 0, height: -0.5)
        return [
            .font: NSFont.monospacedDigitSystemFont(ofSize: 12, weight: .semibold),
            .foregroundColor: color,
            .shadow: shadow,
        ]
    }

    @objc
    private func togglePopover() {
        guard let button = statusItem.button else { return }
        if popover.isShown {
            popover.performClose(nil)
            return
        }

        let clickLocation = NSEvent.mouseLocation
        popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
        NSApplication.shared.activate(ignoringOtherApps: true)
        positionPopover(near: clickLocation, fallbackScreen: button.window?.screen)

        DispatchQueue.main.async { [weak self] in
            self?.positionPopover(near: clickLocation, fallbackScreen: button.window?.screen)
        }
    }

    private func positionPopover(near clickLocation: NSPoint, fallbackScreen: NSScreen?) {
        guard let window = popover.contentViewController?.view.window,
              let screen = NSScreen.screens.first(where: { $0.frame.contains(clickLocation) }) ?? fallbackScreen else {
            return
        }

        // 多显示器或竖屏下 AppKit 偶尔会把弹窗放到屏幕中部，这里按本次点击所在屏幕重新贴近菜单栏。
        let visibleFrame = screen.visibleFrame
        let horizontalPadding: CGFloat = 8
        let minimumX = visibleFrame.minX + horizontalPadding
        let maximumX = visibleFrame.maxX - window.frame.width - horizontalPadding
        let centeredX = clickLocation.x - window.frame.width / 2

        var frame = window.frame
        frame.origin.x = min(max(centeredX, minimumX), max(minimumX, maximumX))
        frame.origin.y = visibleFrame.maxY - frame.height
        window.setFrame(frame, display: true)
    }
}

private struct StatusTitleSegment {
    let text: String
    let color: NSColor
}
