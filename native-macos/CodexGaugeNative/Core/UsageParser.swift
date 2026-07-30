import Foundation

public enum UsageParser {
    public static func parseAppServer(
        account: JSONValue?,
        rateLimits: JSONValue,
        credits: UsageCredits? = nil
    ) -> CodexUsageSnapshot {
        let root = rateLimits.unwrappedResult
        let windows = collectAppServerWindows(root)
        let primary = windows.first { $0.name == "5h" }
        let secondary = windows.first { $0.name == "weekly" }
        let planType = firstString(in: root, keys: ["planType", "plan_type"])
            ?? account.map { firstString(in: $0.unwrappedResult, keys: ["planType", "plan_type", "plan"]) }
            ?? nil
        let status: SnapshotStatus = primary == nil && secondary == nil ? .requestFailed : .ok

        return CodexUsageSnapshot(
            source: .appServer,
            status: status,
            planType: planType,
            primaryWindow: primary,
            primaryWindowUnlimited: status == .ok && primary == nil && secondary != nil,
            secondaryWindow: secondary,
            credits: credits,
            rateLimitReachedType: firstString(in: root, keys: ["rateLimitReachedType"])
        )
    }

    public static func parseWhamUsage(
        _ value: JSONValue,
        credits: UsageCredits? = nil,
        fallbackPlanType: String? = nil
    ) -> CodexUsageSnapshot {
        let root = value.unwrappedResult
        var windows: [UsageWindow] = []
        collectWhamWindows(root, output: &windows)
        let primary = windows.first { $0.name == "5h" }
        let secondary = windows.first { $0.name == "weekly" }

        return CodexUsageSnapshot(
            source: .authJSON,
            status: .ok,
            planType: firstString(in: root, keys: ["plan_type", "planType", "plan"]) ?? fallbackPlanType,
            primaryWindow: primary,
            primaryWindowUnlimited: primary == nil && secondary != nil,
            secondaryWindow: secondary,
            credits: credits ?? parseResetCredits(root),
            rateLimitReachedType: firstString(in: root, keys: ["rateLimitReachedType"])
        )
    }

    public static func parseResetCredits(_ value: JSONValue) -> UsageCredits? {
        let root = value.unwrappedResult
        let countRoot = root["rateLimitResetCredits"] ?? root
        var items = collectCreditItems(root)
        if countRoot != root {
            items.append(contentsOf: collectCreditItems(countRoot))
        }

        let credits = UsageCredits(
            remaining: firstInt(in: countRoot, keys: ["remaining", "remainingCount"]),
            availableCount: firstInt(
                in: countRoot,
                keys: ["available_count", "availableCount", "available", "availableCredits"]
            ),
            resetCredits: firstInt(
                in: countRoot,
                keys: ["availableCount", "available_count", "resetCredits", "reset_credits"]
            ),
            resetAt: firstTimestamp(
                in: countRoot,
                keys: ["resetAt", "reset_at", "resetsAt", "resets_at"]
            ),
            items: items
        )

        if credits.remaining == nil,
           credits.availableCount == nil,
           credits.resetCredits == nil,
           credits.resetAt == nil,
           credits.items.isEmpty {
            return nil
        }
        return credits
    }

    public static func firstString(in value: JSONValue, keys: Set<String>) -> String? {
        if let object = value.objectValue {
            for key in keys {
                if let text = object[key]?.stringValue {
                    return text
                }
            }
            for child in object.values {
                if let text = firstString(in: child, keys: keys) {
                    return text
                }
            }
        }
        if let values = value.arrayValue {
            for child in values {
                if let text = firstString(in: child, keys: keys) {
                    return text
                }
            }
        }
        return nil
    }

    private static func collectAppServerWindows(_ value: JSONValue) -> [UsageWindow] {
        var windows: [UsageWindow] = []
        if let byIdentifier = value["rateLimitsByLimitId"] {
            collectAppServerWindows(from: byIdentifier, output: &windows)
        }
        if windows.isEmpty, let rateLimits = value["rateLimits"] {
            collectAppServerWindows(from: rateLimits, output: &windows)
        }
        for key in ["primary", "secondary"] {
            if let candidate = value[key], let window = parseAppServerWindow(candidate) {
                windows.append(window)
            }
        }
        return windows
    }

    private static func collectAppServerWindows(from value: JSONValue, output: inout [UsageWindow]) {
        if let window = parseAppServerWindow(value) {
            output.append(window)
            return
        }
        if let primary = value["primary"], let window = parseAppServerWindow(primary) {
            output.append(window)
        }
        if let secondary = value["secondary"], let window = parseAppServerWindow(secondary) {
            output.append(window)
        }
        if let nested = value["rateLimits"] {
            collectAppServerWindows(from: nested, output: &output)
            return
        }
        value.objectValue?.values.forEach { collectAppServerWindows(from: $0, output: &output) }
        value.arrayValue?.forEach { collectAppServerWindows(from: $0, output: &output) }
    }

    private static func parseAppServerWindow(_ value: JSONValue) -> UsageWindow? {
        let durationMinutes = value["windowDurationMins"]?.int64Value
        let usedPercent = value["usedPercent"]?.doubleValue
        let resetAt = value["resetsAt"]?.int64Value
        guard durationMinutes != nil || usedPercent != nil || resetAt != nil else { return nil }

        let name: String
        if let durationMinutes, 240 ... 360 ~= durationMinutes {
            name = "5h"
        } else if let durationMinutes, 9_000 ... 11_000 ~= durationMinutes {
            name = "weekly"
        } else {
            name = "other"
        }

        return UsageWindow(
            name: name,
            usedPercent: usedPercent,
            remainingPercent: usedPercent.map { min(100, max(0, 100 - $0)) },
            resetAt: resetAt,
            windowDurationSeconds: durationMinutes.map { $0 * 60 }
        )
    }

    private static func collectWhamWindows(_ value: JSONValue, output: inout [UsageWindow]) {
        if let window = parseWhamWindow(value) {
            output.append(window)
            return
        }
        value.objectValue?.values.forEach { collectWhamWindows($0, output: &output) }
        value.arrayValue?.forEach { collectWhamWindows($0, output: &output) }
    }

    private static func parseWhamWindow(_ value: JSONValue) -> UsageWindow? {
        let duration = firstInt(
            in: value,
            keys: [
                "limit_window_seconds",
                "limitWindowSeconds",
                "windowDurationSeconds",
                "window_duration_seconds",
            ],
            recursive: false
        )
        let name: String
        switch duration {
        case 18_000:
            name = "5h"
        case 604_800:
            name = "weekly"
        default:
            return nil
        }

        let rawUsed = firstDouble(
            in: value,
            keys: [
                "usedPercent",
                "used_percent",
                "usagePercent",
                "usage_percent",
                "currentUsagePercent",
            ],
            recursive: false
        ).map(normalizePercent)
        let rawRemaining = firstDouble(
            in: value,
            keys: [
                "remainingPercent",
                "remaining_percent",
                "remainingPercentage",
                "remaining_percentage",
            ],
            recursive: false
        ).map(normalizePercent)
        let used = rawUsed ?? rawRemaining.map { 100 - $0 }
        let remaining = rawRemaining ?? used.map { min(100, max(0, 100 - $0)) }

        return UsageWindow(
            name: name,
            usedPercent: used,
            remainingPercent: remaining,
            resetAt: firstTimestamp(
                in: value,
                keys: ["resetAt", "reset_at", "resetsAt", "resets_at", "expiresAt", "expires_at"],
                recursive: false
            ),
            windowDurationSeconds: duration
        )
    }

    private static func collectCreditItems(_ value: JSONValue) -> [ResetCreditItem] {
        let candidateKeys = [
            "credits",
            "items",
            "data",
            "resetCredits",
            "reset_credits",
            "rateLimitResetCredits",
        ]
        let values: [JSONValue]?
        if let array = value.arrayValue {
            values = array
        } else {
            values = candidateKeys.compactMap { value[$0]?.arrayValue }.first
        }

        return values?.compactMap { item in
            let credit = ResetCreditItem(
                status: firstString(in: item, keys: ["status", "state"]),
                title: firstString(
                    in: item,
                    keys: ["title", "displayTitle", "display_title", "name"]
                ),
                grantedAt: firstLocalTime(
                    in: item,
                    keys: ["granted_at", "grantedAt", "created_at", "createdAt"]
                ),
                expiresAt: firstLocalTime(
                    in: item,
                    keys: ["expires_at", "expiresAt", "expiration_at", "expirationAt"]
                )
            )
            return credit.id.isEmpty ? nil : credit
        } ?? []
    }

    private static func firstInt(
        in value: JSONValue,
        keys: Set<String>,
        recursive: Bool = true
    ) -> Int64? {
        firstDouble(in: value, keys: keys, recursive: recursive).map(Int64.init)
    }

    private static func firstDouble(
        in value: JSONValue,
        keys: Set<String>,
        recursive: Bool = true
    ) -> Double? {
        if let object = value.objectValue {
            for key in keys {
                if let number = object[key]?.doubleValue {
                    return number
                }
            }
            if recursive {
                for child in object.values {
                    if let number = firstDouble(in: child, keys: keys) {
                        return number
                    }
                }
            }
        }
        if recursive, let values = value.arrayValue {
            for child in values {
                if let number = firstDouble(in: child, keys: keys) {
                    return number
                }
            }
        }
        return nil
    }

    private static func firstTimestamp(
        in value: JSONValue,
        keys: Set<String>,
        recursive: Bool = true
    ) -> Int64? {
        if let number = firstDouble(in: value, keys: keys, recursive: recursive) {
            return Int64(number)
        }
        if let text = firstString(in: value, keys: keys) {
            let formatter = ISO8601DateFormatter()
            if let date = formatter.date(from: text) {
                return Int64(date.timeIntervalSince1970)
            }
        }
        return nil
    }

    private static func firstLocalTime(in value: JSONValue, keys: Set<String>) -> String? {
        guard let timestamp = firstTimestamp(in: value, keys: keys) else { return nil }
        return Date(timeIntervalSince1970: TimeInterval(timestamp)).formatted(
            .dateTime.month(.twoDigits).day(.twoDigits).hour(.twoDigits(amPM: .omitted)).minute()
        )
    }

    private static func normalizePercent(_ value: Double) -> Double {
        let percent = value >= 0 && value <= 1 ? value * 100 : value
        return min(100, max(0, percent))
    }
}
