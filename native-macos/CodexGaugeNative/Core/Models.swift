import Foundation

public enum SnapshotSource: String, Codable, Sendable {
    case authJSON = "auth-json"
    case appServer = "app-server"
    case sessionLog = "session-log"
}

public enum SnapshotStatus: String, Codable, Sendable {
    case ok
    case notLoggedIn = "not_logged_in"
    case invalidAuth = "invalid_auth"
    case requestFailed = "request_failed"
}

public struct UsageWindow: Codable, Equatable, Sendable {
    public let name: String
    public let usedPercent: Double?
    public let remainingPercent: Double?
    public let resetAt: Int64?
    public let windowDurationSeconds: Int64?

    public init(
        name: String,
        usedPercent: Double?,
        remainingPercent: Double?,
        resetAt: Int64?,
        windowDurationSeconds: Int64?
    ) {
        self.name = name
        self.usedPercent = usedPercent
        self.remainingPercent = remainingPercent
        self.resetAt = resetAt
        self.windowDurationSeconds = windowDurationSeconds
    }
}

public enum RemainingStatus: Equatable, Sendable {
    case unknown
    case empty
    case critical
    case low
    case mid
    case good
    case full

    public init(remainingPercent: Double?) {
        guard let remainingPercent else {
            self = .unknown
            return
        }
        switch remainingPercent {
        case ...5:
            self = .empty
        case ...15:
            self = .critical
        case ...30:
            self = .low
        case ...50:
            self = .mid
        case ...70:
            self = .good
        default:
            self = .full
        }
    }
}

public struct ResetCreditItem: Codable, Equatable, Identifiable, Sendable {
    public let status: String?
    public let title: String?
    public let grantedAt: String?
    public let expiresAt: String?

    public var id: String {
        [title, grantedAt, expiresAt, status].compactMap { $0 }.joined(separator: "|")
    }

    public init(status: String?, title: String?, grantedAt: String?, expiresAt: String?) {
        self.status = status
        self.title = title
        self.grantedAt = grantedAt
        self.expiresAt = expiresAt
    }
}

public struct UsageCredits: Codable, Equatable, Sendable {
    public let remaining: Int64?
    public let availableCount: Int64?
    public let resetCredits: Int64?
    public let resetAt: Int64?
    public let items: [ResetCreditItem]

    public init(
        remaining: Int64?,
        availableCount: Int64?,
        resetCredits: Int64?,
        resetAt: Int64?,
        items: [ResetCreditItem]
    ) {
        self.remaining = remaining
        self.availableCount = availableCount
        self.resetCredits = resetCredits
        self.resetAt = resetAt
        self.items = items
    }

    public var availableResetCount: Int64? {
        if let aggregate = availableCount ?? resetCredits ?? remaining {
            return aggregate
        }
        let recognizedStatuses = items.compactMap { item -> String? in
            guard let status = item.status?.lowercased(),
                  ["available", "active", "consumed", "used", "expired"].contains(status) else {
                return nil
            }
            return status
        }
        if !recognizedStatuses.isEmpty {
            return Int64(recognizedStatuses.count { $0 == "available" || $0 == "active" })
        }
        return nil
    }
}

public struct CodexUsageSnapshot: Codable, Equatable, Sendable {
    public let source: SnapshotSource
    public let status: SnapshotStatus
    public let planType: String?
    public let primaryWindow: UsageWindow?
    public let primaryWindowUnlimited: Bool
    public let secondaryWindow: UsageWindow?
    public let credits: UsageCredits?
    public let rateLimitReachedType: String?
    public let updatedAt: Int64

    public init(
        source: SnapshotSource,
        status: SnapshotStatus,
        planType: String? = nil,
        primaryWindow: UsageWindow? = nil,
        primaryWindowUnlimited: Bool = false,
        secondaryWindow: UsageWindow? = nil,
        credits: UsageCredits? = nil,
        rateLimitReachedType: String? = nil,
        updatedAt: Int64 = Int64(Date().timeIntervalSince1970)
    ) {
        self.source = source
        self.status = status
        self.planType = planType
        self.primaryWindow = primaryWindow
        self.primaryWindowUnlimited = primaryWindowUnlimited
        self.secondaryWindow = secondaryWindow
        self.credits = credits
        self.rateLimitReachedType = rateLimitReachedType
        self.updatedAt = updatedAt
    }

    public static func empty(
        source: SnapshotSource,
        status: SnapshotStatus
    ) -> CodexUsageSnapshot {
        CodexUsageSnapshot(source: source, status: status)
    }

    public var hasCompleteUsage: Bool {
        status == .ok && (primaryWindow != nil || secondaryWindow != nil)
    }
}

public enum PreferredProvider: String, Codable, CaseIterable, Sendable {
    case appServer = "app-server"
    case api
}

public enum MenuBarDisplay: String, Codable, CaseIterable, Sendable {
    case fiveAndSeven
    case fiveHour
    case iconOnly
}

public struct AppConfig: Codable, Equatable, Sendable {
    public var refreshIntervalSeconds: Int
    public var startOnBoot: Bool
    public var command: String
    public var preferredProvider: PreferredProvider
    public var menuBarDisplay: MenuBarDisplay
    public var automaticallyChecksForUpdates: Bool

    public init(
        refreshIntervalSeconds: Int = 60,
        startOnBoot: Bool = false,
        command: String = "codex",
        preferredProvider: PreferredProvider = .appServer,
        menuBarDisplay: MenuBarDisplay = .fiveHour,
        automaticallyChecksForUpdates: Bool = true
    ) {
        self.refreshIntervalSeconds = refreshIntervalSeconds
        self.startOnBoot = startOnBoot
        self.command = command
        self.preferredProvider = preferredProvider
        self.menuBarDisplay = menuBarDisplay
        self.automaticallyChecksForUpdates = automaticallyChecksForUpdates
    }
}
