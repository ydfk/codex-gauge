import SwiftUI

struct MenuBarPanel: View {
    @ObservedObject var model: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [
                    Color.accentColor.opacity(0.09),
                    Color.mint.opacity(0.05),
                    .clear,
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            VStack(spacing: 0) {
                header
                Divider().opacity(0.55)

                Group {
                    switch model.page {
                    case .usage:
                        UsageOverview(model: model)
                    case .settings:
                        SettingsPanel(model: model)
                    }
                }
            }
        }
        .frame(width: 376, height: 510)
        .task {
            await model.startIfNeeded()
        }
    }

    private var header: some View {
        HStack(spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(model.page == .usage ? "Codex Gauge" : "设置")
                    .font(.system(size: 15, weight: .semibold))
                Text(model.statusText)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()

            Button {
                if model.page == .usage {
                    Task { await model.refresh() }
                } else {
                    withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) {
                        model.page = .usage
                    }
                }
            } label: {
                Image(systemName: model.page == .usage ? "arrow.clockwise" : "chevron.left")
                    .font(.system(size: 13, weight: .semibold))
                    .frame(width: 18, height: 18)
                    .rotationEffect(model.isRefreshing && !reduceMotion ? .degrees(360) : .zero)
                    .animation(
                        model.isRefreshing && !reduceMotion
                            ? .linear(duration: 0.8).repeatForever(autoreverses: false)
                            : .default,
                        value: model.isRefreshing
                    )
            }
            .gaugeButtonStyle()
            .disabled(model.isRefreshing)
            .help(model.page == .usage ? "刷新用量" : "返回用量")
        }
        .padding(.horizontal, 16)
        .frame(height: 60)
    }
}

private struct UsageOverview: View {
    @ObservedObject var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 8) {
                if model.needsLogin {
                    loginNotice
                }

                GlassGroup {
                    VStack(spacing: 8) {
                        UsageCard(
                            title: "5 小时",
                            subtitle: "短周期额度",
                            window: model.snapshot?.primaryWindow,
                            unlimited: model.snapshot?.primaryWindowUnlimited == true
                        )
                        UsageCard(
                            title: "7 天",
                            subtitle: "长周期额度",
                            window: model.snapshot?.secondaryWindow,
                            unlimited: false
                        )
                    }
                }

                facts

                if let credits = model.snapshot?.credits?.items, !credits.isEmpty {
                    creditList(credits)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .frame(maxHeight: .infinity, alignment: .top)

            Divider().opacity(0.55)
            footer
        }
    }

    private var loginNotice: some View {
        Button(action: model.openCodexLogin) {
            HStack {
                Label("未检测到有效登录状态", systemImage: "person.crop.circle.badge.exclamationmark")
                Spacer()
                Text("打开 Codex 登录")
                    .fontWeight(.semibold)
            }
            .font(.caption)
            .padding(12)
            .frame(maxWidth: .infinity)
        }
        .gaugeButtonStyle(prominent: true)
    }

    private var facts: some View {
        HStack(spacing: 0) {
            FactCell(
                title: "可用重置",
                value: model.snapshot?.credits?.availableResetCount.map(String.init) ?? "未知"
            )
            Divider().frame(height: 34)
            FactCell(title: "当前计划", value: model.snapshot?.planType ?? "未知")
            Divider().frame(height: 34)
            FactCell(title: "数据来源", value: model.sourceText)
        }
        .padding(.vertical, 6)
        .gaugeGlass(cornerRadius: 14)
    }

    private func creditList(_ credits: [ResetCreditItem]) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Text("重置券明细 · 共 \(credits.count) 张")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
                .textCase(.uppercase)

            ForEach(Array(credits.prefix(2).enumerated()), id: \.offset) { _, credit in
                HStack(alignment: .center, spacing: 8) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(creditTitle(credit.title))
                            .font(.caption.weight(.semibold))
                        Text(credit.expiresAt.map { "到期 \($0)" } ?? "未提供到期时间")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    Text(creditStatus(credit.status))
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(creditStatusColor(credit.status))
                }
                .padding(.horizontal, 9)
                .frame(height: 38)
                .gaugeGlass(cornerRadius: 10)
            }
        }
    }

    private var footer: some View {
        HStack {
            Text(updatedText)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Spacer()
            Button("设置") {
                withAnimation(.easeOut(duration: 0.2)) {
                    model.page = .settings
                }
            }
            .gaugeButtonStyle()
            Button("退出", action: model.quit)
                .gaugeButtonStyle()
        }
        .padding(.horizontal, 14)
        .frame(height: 48)
    }

    private var updatedText: String {
        guard let timestamp = model.snapshot?.updatedAt else { return "尚未更新" }
        let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
        return "更新于 \(date.formatted(.dateTime.hour(.twoDigits(amPM: .omitted)).minute().second()))"
    }

    private func creditTitle(_ title: String?) -> String {
        guard let title else { return "重置券" }
        return title.localizedCaseInsensitiveContains("full reset")
            ? "完整重置（5h + 7d）"
            : title
    }

    private func creditStatus(_ status: String?) -> String {
        switch status?.lowercased() {
        case "available", "active":
            "可用"
        case "consumed", "used":
            "已使用"
        case "expired":
            "已过期"
        default:
            status ?? "未知"
        }
    }

    private func creditStatusColor(_ status: String?) -> Color {
        switch status?.lowercased() {
        case "available", "active":
            RemainingStatus.full.color
        case "consumed", "used", "expired":
            .secondary
        default:
            RemainingStatus.mid.color
        }
    }
}

private struct FactCell: View {
    let title: String
    let value: String

    var body: some View {
        VStack(spacing: 3) {
            Text(title)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.caption.weight(.semibold))
                .lineLimit(1)
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .combine)
    }
}
