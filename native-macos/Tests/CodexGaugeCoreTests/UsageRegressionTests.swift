import CodexGaugeCore
import Foundation
import Testing

private func payload(_ text: String) throws -> JSONValue {
    try JSONDecoder().decode(JSONValue.self, from: Data(text.utf8))
}

@Test("多额度池只读取 Codex 的用量与重置时间")
func selectsCodexPool() throws {
    let value = try payload("""
    {"rateLimitsByLimitId": {
      "other-model": {"primary":{"usedPercent":0,"windowDurationMins":300,"resetsAt":1900000000}},
      "codex": {
        "primary":{"usedPercent":43,"windowDurationMins":300,"resetsAt":1790959614},
        "secondary":{"usedPercent":31,"windowDurationMins":10080,"resetsAt":1791463406}
      }
    }}
    """)
    let snapshot = UsageParser.parseAppServer(account: nil, rateLimits: value)
    #expect(snapshot.primaryWindow?.remainingPercent == 57)
    #expect(snapshot.primaryWindow?.resetAt == 1790959614)
    #expect(snapshot.secondaryWindow?.remainingPercent == 69)
    #expect(snapshot.secondaryWindow?.resetAt == 1791463406)
}

@Test("未知模型额度池不会覆盖旧接口主额度")
func fallsBackToLegacyPool() throws {
    let value = try payload("""
    {"rateLimitsByLimitId": {
      "other-model": {"primary":{"usedPercent":0,"windowDurationMins":300}}
    }, "rateLimits":{"primary":{"usedPercent":43,"windowDurationMins":300}}}
    """)
    #expect(UsageParser.parseAppServer(account: nil, rateLimits: value).primaryWindow?.remainingPercent == 57)
}

@Test("Wham 主额度不会混入代码审查或附加模型的额度")
func selectsWhamMainPool() throws {
    let value = try payload("""
    {
      "rate_limit": {
        "primary_window":{"used_percent":45,"limit_window_seconds":18000,"reset_at":1790959614},
        "secondary_window":{"used_percent":31,"limit_window_seconds":604800,"reset_at":1791463406}
      },
      "code_review_rate_limit":{"secondary_window":{"used_percent":0,"limit_window_seconds":604800,"reset_at":1900000000}},
      "additional_rate_limits":[{"primary_window":{"used_percent":0,"limit_window_seconds":18000}}]
    }
    """)
    let snapshot = UsageParser.parseWhamUsage(value)
    #expect(snapshot.primaryWindow?.remainingPercent == 55)
    #expect(snapshot.secondaryWindow?.remainingPercent == 69)
    #expect(snapshot.secondaryWindow?.resetAt == 1791463406)
    #expect(snapshot.credits == nil)
}

@Test("百分数 1 和小数百分数不放大一百倍", arguments: [0.0, 0.75, 1.0])
func preservesPercentUnits(used: Double) throws {
    let value = try payload("""
    {"rate_limit":{"primary_window":{"used_percent":\(used),"limit_window_seconds":18000}}}
    """)
    let snapshot = UsageParser.parseWhamUsage(value)
    #expect(snapshot.primaryWindow?.usedPercent == used)
    #expect(snapshot.primaryWindow?.remainingPercent == 100 - used)
}

@Test("空用量响应不能标记成功")
func rejectsEmptyUsage() throws {
    #expect(UsageParser.parseWhamUsage(try payload("{}")).status == .requestFailed)
    let value = try payload("""
    {"rate_limit":null,"code_review_rate_limit":{"secondary_window":{"used_percent":0,"limit_window_seconds":604800}}}
    """)
    #expect(UsageParser.parseWhamUsage(value).status == .requestFailed)
}

@Test("重置券支持带微秒的 UTC 时间并转换为本地时间")
func parsesFractionalCreditDates() throws {
    let value = try payload("""
    {"available_count":3,"credits":[
      {"status":"available","title":"Full reset","expires_at":"2026-10-04T22:03:26.066339Z"},
      {"status":"available","expires_at":"2026-10-04T22:03:26Z"},
      {"status":"available","expires_at":1791151406066}
    ]}
    """)
    let credits = try #require(UsageParser.parseResetCredits(value))
    let expected = Date(timeIntervalSince1970: 1791151406).formatted(
        .dateTime.month(.twoDigits).day(.twoDigits).hour(.twoDigits(amPM: .omitted)).minute()
    )
    #expect(credits.availableResetCount == 3)
    #expect(credits.items.count == 3)
    #expect(credits.items.allSatisfy { $0.expiresAt == expected })
}

@Test("App Server 优先使用响应内的重置券")
func parsesEmbeddedCredits() throws {
    let value = try payload("""
    {"rateLimits":{"primary":{"usedPercent":43,"windowDurationMins":300}},
     "rateLimitResetCredits":{"availableCount":3,"credits":[{"status":"available"}]}}
    """)
    let snapshot = UsageParser.parseAppServer(account: nil, rateLimits: value)
    #expect(snapshot.credits?.availableResetCount == 3)
    #expect(snapshot.credits?.items.count == 1)
}
