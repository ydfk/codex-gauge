import Foundation

public enum MenuBarPresentation {
    public static func title(
        snapshot: CodexUsageSnapshot?,
        mode: MenuBarDisplay
    ) -> String {
        guard mode != .iconOnly else { return "" }

        if snapshot?.primaryWindowUnlimited == true {
            return "7d \(percent(snapshot?.secondaryWindow?.remainingPercent))"
        }

        let fiveHour = "5h \(percent(snapshot?.primaryWindow?.remainingPercent))"
        let weekly = "7d \(percent(snapshot?.secondaryWindow?.remainingPercent))"
        return "\(fiveHour) · \(weekly)"
    }

    private static func percent(_ value: Double?) -> String {
        value.map { "\(Int($0.rounded()))%" } ?? "--"
    }
}
