import SwiftUI

struct SettingsPanel: View {
    @ObservedObject var model: AppModel

    var body: some View {
        ScrollView {
            VStack(spacing: 14) {
                settingsSection("菜单栏") {
                    settingRow("显示内容") {
                        Picker("", selection: model.binding(for: \.menuBarDisplay)) {
                            Text("图标 + 5h + 7d").tag(MenuBarDisplay.fiveAndSeven)
                            Text("图标 + 5h").tag(MenuBarDisplay.fiveHour)
                            Text("仅图标").tag(MenuBarDisplay.iconOnly)
                        }
                        .labelsHidden()
                        .frame(width: 164)
                    }

                    Divider()

                    settingRow("刷新间隔") {
                        Picker("", selection: model.binding(for: \.refreshIntervalSeconds)) {
                            Text("30 秒").tag(30)
                            Text("1 分钟").tag(60)
                            Text("2 分钟").tag(120)
                            Text("5 分钟").tag(300)
                            Text("10 分钟").tag(600)
                        }
                        .labelsHidden()
                        .frame(width: 164)
                    }

                    Divider()

                    Toggle(
                        "登录时启动",
                        isOn: Binding(
                            get: { model.config.startOnBoot },
                            set: model.setStartOnBoot
                        )
                    )
                }

                settingsSection("Codex") {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("命令路径")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        TextField(
                            "codex",
                            text: model.binding(for: \.command)
                        )
                        .textFieldStyle(.roundedBorder)
                    }

                    Divider()

                    settingRow("优先查询方式") {
                        Picker("", selection: model.binding(for: \.preferredProvider)) {
                            Text("App Server").tag(PreferredProvider.appServer)
                            Text("AuthJson API").tag(PreferredProvider.api)
                        }
                        .labelsHidden()
                        .frame(width: 164)
                    }
                }

                settingsSection("更新") {
                    Toggle(
                        "自动检查更新",
                        isOn: model.binding(for: \.automaticallyChecksForUpdates)
                    )
                    .disabled(!model.updater.isConfigured)

                    Divider()

                    HStack {
                        VStack(alignment: .leading, spacing: 3) {
                            Text("Sparkle 安全更新")
                                .font(.caption.weight(.semibold))
                            Text(updateDescription)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Button("检查更新") {
                            model.updater.checkForUpdates()
                        }
                        .gaugeButtonStyle()
                        .disabled(!model.updater.isConfigured)
                    }
                }

                Text("Codex 登录凭据只在内存中用于请求，不会写入原生应用配置。")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(14)
        }
    }

    private var updateDescription: String {
        if model.updater.isConfigured {
            return "通过签名 appcast 自动检查并安装"
        }
        return "发布前需在 Local.xcconfig 配置 Ed25519 公钥"
    }

    private func settingsSection<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            Text(title)
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
                .textCase(.uppercase)
                .padding(.leading, 3)

            VStack(spacing: 10) {
                content()
            }
            .padding(12)
            .gaugeGlass(cornerRadius: 15)
        }
    }

    private func settingRow<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        HStack {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            content()
        }
    }
}
