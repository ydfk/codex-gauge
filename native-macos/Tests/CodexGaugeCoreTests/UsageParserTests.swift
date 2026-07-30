import CodexGaugeCore
import Foundation
import Testing

@Test("解析 App Server 的 5h 与周额度")
func parsesAppServerWindows() throws {
    let value = try decode(
        """
        {
          "result": {
            "rateLimitsByLimitId": {
              "short": { "usedPercent": 42, "windowDurationMins": 300, "resetsAt": 1900000000 },
              "week": { "usedPercent": 17, "windowDurationMins": 10080, "resetsAt": 1900000000 }
            }
          }
        }
        """
    )

    let snapshot = UsageParser.parseAppServer(account: nil, rateLimits: value)

    #expect(snapshot.primaryWindow?.usedPercent == 42)
    #expect(snapshot.primaryWindow?.remainingPercent == 58)
    #expect(snapshot.secondaryWindow?.usedPercent == 17)
}

@Test("成功查询但缺少 5h 时识别为无限")
func marksMissingFiveHourAsUnlimited() throws {
    let value = try decode(
        """
        {
          "rateLimits": [
            { "usedPercent": 12, "windowDurationMins": 10080, "resetsAt": 1900000000 }
          ]
        }
        """
    )

    let snapshot = UsageParser.parseAppServer(account: nil, rateLimits: value)

    #expect(snapshot.status == .ok)
    #expect(snapshot.primaryWindowUnlimited)
}

@Test("5h 无限时菜单栏回退显示 7d")
func fallsBackToWeeklyMenuBarTitle() throws {
    let value = try decode(
        """
        {
          "rateLimits": [
            { "usedPercent": 42, "windowDurationMins": 10080, "resetsAt": 1900000000 }
          ]
        }
        """
    )
    let snapshot = UsageParser.parseAppServer(account: nil, rateLimits: value)

    #expect(MenuBarPresentation.title(snapshot: snapshot, mode: .fiveHour) == "7d 58%")
    #expect(MenuBarPresentation.title(snapshot: snapshot, mode: .fiveAndSeven) == "7d 58%")
}

@Test("5h 与 7d 都没有可显示额度时菜单栏显示无限")
func showsUnlimitedWhenWeeklyIsAlsoUnavailable() {
    let snapshot = CodexUsageSnapshot(
        source: .appServer,
        status: .ok,
        primaryWindowUnlimited: true
    )

    #expect(MenuBarPresentation.title(snapshot: snapshot, mode: .fiveHour) == "无限")
}

@Test("可用重置次数优先使用接口汇总")
func prefersAggregateResetCount() {
    let credits = UsageCredits(
        remaining: nil,
        availableCount: 3,
        resetCredits: 3,
        resetAt: nil,
        items: [
            ResetCreditItem(status: "available", title: "Full reset", grantedAt: nil, expiresAt: nil),
            ResetCreditItem(status: "consumed", title: "Full reset", grantedAt: nil, expiresAt: nil),
        ]
    )

    #expect(credits.availableResetCount == 3)
}

@Test("缺少接口汇总时回退统计可用明细")
func fallsBackToAvailableItems() {
    let credits = UsageCredits(
        remaining: nil,
        availableCount: nil,
        resetCredits: nil,
        resetAt: nil,
        items: [
            ResetCreditItem(status: "available", title: "Full reset", grantedAt: nil, expiresAt: nil),
            ResetCreditItem(status: "consumed", title: "Full reset", grantedAt: nil, expiresAt: nil),
        ]
    )

    #expect(credits.availableResetCount == 1)
}

@Test("相同内容的两张重置券都保留")
func preservesDuplicateCreditItems() throws {
    let value = try decode(
        """
        {
          "available_count": 2,
          "credits": [
            { "title": "Full reset", "status": "available" },
            { "title": "Full reset", "status": "available" }
          ]
        }
        """
    )

    #expect(UsageParser.parseResetCredits(value)?.items.count == 2)
}

@Test("剩余百分比沿用旧版六档状态")
func classifiesRemainingStatus() {
    #expect(RemainingStatus(remainingPercent: nil) == .unknown)
    #expect(RemainingStatus(remainingPercent: 5) == .empty)
    #expect(RemainingStatus(remainingPercent: 15) == .critical)
    #expect(RemainingStatus(remainingPercent: 30) == .low)
    #expect(RemainingStatus(remainingPercent: 50) == .mid)
    #expect(RemainingStatus(remainingPercent: 70) == .good)
    #expect(RemainingStatus(remainingPercent: 71) == .full)
}

@Test("解析 AuthJson API 的小数百分比与重置券")
func parsesWhamUsageAndCredits() throws {
    let value = try decode(
        """
        {
          "result": {
            "planType": "plus",
            "limits": [
              {
                "limit_window_seconds": 18000,
                "remaining_percent": 0.75,
                "reset_at": 1900000000
              },
              {
                "limit_window_seconds": 604800,
                "used_percent": 30,
                "reset_at": 1900000000
              }
            ],
            "rateLimitResetCredits": {
              "availableCount": 2,
              "items": [
                { "title": "Full reset", "status": "available" }
              ]
            }
          }
        }
        """
    )

    let snapshot = UsageParser.parseWhamUsage(value)

    #expect(snapshot.primaryWindow?.remainingPercent == 75)
    #expect(snapshot.primaryWindow?.usedPercent == 25)
    #expect(snapshot.secondaryWindow?.remainingPercent == 70)
    #expect(snapshot.credits?.availableCount == 2)
}

private func decode(_ text: String) throws -> JSONValue {
    try JSONDecoder().decode(JSONValue.self, from: Data(text.utf8))
}
