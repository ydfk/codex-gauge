import CodexGaugeCore
import Foundation
import Testing

@Test("有限 5h 同时显示周额度，兼容旧显示模式")
func showsBothLimitedWindows() {
    for mode in [MenuBarDisplay.fiveHour, .fiveAndSeven] {
        for remaining in [0.0, 9.0, 50.0, 100.0] {
            let snapshot = menuBarSnapshot(five: remaining, weekly: 100)
            #expect(MenuBarPresentation.title(snapshot: snapshot, mode: mode)
                == "5h \(Int(remaining))% · 7d 100%")
        }
    }
}

@Test("缺失额度使用占位且不隐藏另一窗口")
func showsMissingMenuBarValues() {
    for mode in [MenuBarDisplay.fiveHour, .fiveAndSeven] {
        #expect(MenuBarPresentation.title(snapshot: nil, mode: mode) == "5h -- · 7d --")
        #expect(MenuBarPresentation.title(snapshot: menuBarSnapshot(five: 50, weekly: nil), mode: mode)
            == "5h 50% · 7d --")
    }
}

@Test("额度状态切换时菜单栏内容增减，图标模式保持无文字")
func menuBarContentAdaptsToWindowAvailability() {
    let limited = menuBarSnapshot(five: 100, weekly: 90)
    let unlimited = menuBarSnapshot(five: nil, weekly: 90, unlimited: true)
    for mode in [MenuBarDisplay.fiveHour, .fiveAndSeven] {
        let titles = [limited, unlimited, limited].map {
            MenuBarPresentation.title(snapshot: $0, mode: mode)
        }
        #expect(titles == ["5h 100% · 7d 90%", "7d 90%", "5h 100% · 7d 90%"])
    }
    for snapshot in [limited, unlimited] {
        #expect(MenuBarPresentation.title(snapshot: snapshot, mode: .iconOnly).isEmpty)
    }
}

@Test("新配置默认显示双窗口，旧配置迁移不丢失个人设置")
func migratesLegacyMenuBarMode() throws {
    #expect(AppConfig().menuBarDisplay == .fiveAndSeven)
    let original = AppConfig(refreshIntervalSeconds: 300, startOnBoot: true, menuBarDisplay: .fiveHour)
    let data = try JSONEncoder().encode(original)
    let restored = try JSONDecoder().decode(AppConfig.self, from: data)
    #expect(restored.menuBarDisplay == .fiveAndSeven)
    #expect(restored.refreshIntervalSeconds == 300)
    #expect(restored.startOnBoot)
    let icon = try JSONDecoder().decode(MenuBarDisplay.self, from: Data("\"iconOnly\"".utf8))
    #expect(icon == .iconOnly)
}

private func menuBarSnapshot(five: Double?, weekly: Double?, unlimited: Bool = false) -> CodexUsageSnapshot {
    CodexUsageSnapshot(
        source: .appServer,
        status: .ok,
        primaryWindow: five.map {
            UsageWindow(name: "5h", usedPercent: 100 - $0, remainingPercent: $0,
                        resetAt: nil, windowDurationSeconds: 18000)
        },
        primaryWindowUnlimited: unlimited,
        secondaryWindow: weekly.map {
            UsageWindow(name: "weekly", usedPercent: 100 - $0, remainingPercent: $0,
                        resetAt: nil, windowDurationSeconds: 604800)
        }
    )
}
