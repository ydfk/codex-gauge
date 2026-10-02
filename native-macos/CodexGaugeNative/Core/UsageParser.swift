import Foundation

public enum UsageParser {
    public static func parseAppServer(
        account: JSONValue?,
        rateLimits: JSONValue,
        credits: UsageCredits? = nil
    ) -> CodexUsageSnapshot {
        let root = rateLimits.unwrappedResult
        let limitRoot = appServerLimitRoot(root)
        let windows = collectAppServerWindows(limitRoot)
        let primary = windows.first { $0.name == "5h" }
        let secondary = windows.first { $0.name == "weekly" }
        let planType = firstString(in: limitRoot, keys: ["planType", "plan_type"])
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
            credits: root["rateLimitResetCredits"].flatMap(parseResetCredits) ?? credits,
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
        // 只读取主额度池，避免混入代码审查或其他模型的额度和重置时间。
        let limitRoot = root["rate_limit"] ?? root["rateLimit"] ?? root["limits"] ?? root
        collectWhamWindows(limitRoot, output: &windows)
        let primary = windows.first { $0.name == "5h" }
        let secondary = windows.first { $0.name == "weekly" }

        return CodexUsageSnapshot(
            source: .authJSON,
            status: primary == nil && secondary == nil ? .requestFailed : .ok,
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
        let items = collectCreditItems(countRoot)

        let credits = UsageCredits(
            remaining: firstInt(in: countRoot, keys: ["remaining", "remainingCount"], recursive: false),
            availableCount: firstInt(
                in: countRoot,
                keys: ["available_count", "availableCount", "available", "availableCredits"],
                recursive: false
            ),
            resetCredits: firstInt(
                in: countRoot,
                keys: ["availableCount", "available_count", "resetCredits", "reset_credits"],
                recursive: false
            ),
            resetAt: firstTimestamp(
                in: countRoot,
                keys: ["resetAt", "reset_at", "resetsAt", "resets_at"],
                recursive: false
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

    public static func firstString(
        in value: JSONValue, keys: Set<String>, recursive: Bool = true
    ) -> String? {
        if let object = value.objectValue {
            for key in keys {
                if let text = object[key]?.stringValue {
                    return text
                }
            }
            guard recursive else { return nil }
            for child in object.values {
                if let text = firstString(in: child, keys: keys) {
                    return text
                }
            }
        }
        if recursive, let values = value.arrayValue {
            for child in values {
                if let text = firstString(in: child, keys: keys) {
                    return text
                }
            }
        }
        return nil
    }

    private static func appServerLimitRoot(_ value: JSONValue) -> JSONValue {
        // 新接口按额度池返回数据，必须优先选择 Codex，不能依赖字典遍历顺序。
        if let codex = value["rateLimitsByLimitId"]?["codex"], codex.objectValue != nil {
            return codex
        }
        if let legacy = value["rateLimits"], legacy != .null {
            return legacy
        }
        if let byIdentifier = value["rateLimitsByLimitId"]?.objectValue {
            // 兼容旧版直接按窗口名称返回的结构，不读取未知模型池。
            return .object(byIdentifier.filter {
                parseAppServerWindow($0.value) != nil
            })
        }
        return value
    }

    private static func collectAppServerWindows(_ value: JSONValue) -> [UsageWindow] {
        var windows: [UsageWindow] = []
        collectAppServerWindows(from: value, output: &windows)
        return windows
    }

    private static func collectAppServerWindows(from value: JSONValue, output: inout [UsageWindow]) {
        if let window = parseAppServerWindow(value) {
            output.append(window)
            return
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
        let resetAt = firstTimestamp(in: value, keys: ["resetsAt"], recursive: false)
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
            // 兼容 Unix 秒和毫秒时间戳。
            let seconds = abs(number) >= 100_000_000_000 ? number / 1_000 : number
            return Int64(exactly: seconds.rounded(.towardZero))
        }
        if let text = firstString(in: value, keys: keys, recursive: recursive) {
            let formatter = ISO8601DateFormatter()
            formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
            let fractionalDate = formatter.date(from: text)
            formatter.formatOptions = [.withInternetDateTime]
            if let date = fractionalDate ?? formatter.date(from: text) {
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
        // percent 字段的单位始终为百分数，1 表示 1%，不是 100%。
        min(100, max(0, value))
    }
}
