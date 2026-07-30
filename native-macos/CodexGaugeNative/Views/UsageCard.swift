import SwiftUI

struct UsageCard: View {
    let title: String
    let subtitle: String
    let window: UsageWindow?
    let unlimited: Bool

    private var remaining: Double? {
        unlimited ? 100 : window?.remainingPercent
    }

    private var accent: Color {
        RemainingStatus(remainingPercent: remaining).color
    }

    var body: some View {
        VStack(spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.system(size: 14, weight: .semibold))
                    Text(subtitle)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text(remainingText)
                    .font(.system(size: 21, weight: .semibold, design: .rounded))
                    .monospacedDigit()
                    .foregroundStyle(accent)
            }

            Gauge(value: remaining ?? 0, in: 0 ... 100) {}
                .gaugeStyle(.accessoryLinearCapacity)
                .tint(accent)
                .accessibilityLabel("\(title)剩余额度")
                .accessibilityValue(remainingText)

            HStack {
                Text(usageText)
                Spacer()
                Text(resetText)
            }
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
        .padding(10)
        .gaugeGlass(cornerRadius: 14, tint: accent.opacity(0.10))
    }

    private var remainingText: String {
        if unlimited {
            return "无限"
        }
        guard let value = window?.remainingPercent else { return "未知" }
        return "\(Int(value.rounded()))%"
    }

    private var usageText: String {
        if unlimited {
            return "无需重置"
        }
        guard let value = window?.usedPercent else { return "已用 未知" }
        return "已用 \(Int(value.rounded()))%"
    }

    private var resetText: String {
        if unlimited {
            return "当前套餐不受限制"
        }
        guard let seconds = window?.resetAt else { return "重置时间未知" }
        let date = Date(timeIntervalSince1970: TimeInterval(seconds))
        return "重置 \(date.formatted(.dateTime.month(.twoDigits).day(.twoDigits).hour(.twoDigits(amPM: .omitted)).minute()))"
    }
}

extension RemainingStatus {
    var color: Color {
        switch self {
        case .unknown:
            .secondary
        case .empty:
            Color(red: 1.00, green: 0.33, blue: 0.43)
        case .critical:
            Color(red: 1.00, green: 0.48, blue: 0.51)
        case .low:
            Color(red: 1.00, green: 0.62, blue: 0.37)
        case .mid:
            Color(red: 1.00, green: 0.77, blue: 0.42)
        case .good:
            Color(red: 0.71, green: 0.93, blue: 0.46)
        case .full:
            Color(red: 0.45, green: 0.89, blue: 0.79)
        }
    }
}
