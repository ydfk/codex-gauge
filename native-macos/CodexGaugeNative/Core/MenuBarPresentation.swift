import Foundation

public enum MenuBarPresentation {
    public static func title(
        snapshot: CodexUsageSnapshot?,
        mode: MenuBarDisplay
    ) -> String {
        guard mode != .iconOnly else { return "" }

        if snapshot?.primaryWindowUnlimited == true {
            guard let weeklyRemaining = snapshot?.secondaryWindow?.remainingPercent else {
                return "无限"
            }
            return "7d \(percent(weeklyRemaining))"
        }

        let fiveHour = "5h \(percent(snapshot?.primaryWindow?.remainingPercent))"
        guard mode == .fiveAndSeven else { return fiveHour }
        let weekly = "7d \(percent(snapshot?.secondaryWindow?.remainingPercent))"
        return "\(fiveHour) · \(weekly)"
    }

    private static func percent(_ value: Double?) -> String {
        value.map { "\(Int($0.rounded()))%" } ?? "--"
    }
}
