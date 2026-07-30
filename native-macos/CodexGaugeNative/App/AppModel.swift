import AppKit
import Combine
import Foundation
import SwiftUI

@MainActor
final class AppModel: ObservableObject {
    enum PanelPage {
        case usage
        case settings
    }

    @Published private(set) var snapshot: CodexUsageSnapshot?
    @Published private(set) var config: AppConfig
    @Published private(set) var isRefreshing = false
    @Published private(set) var message = ""
    @Published var page: PanelPage = .usage

    let updater = UpdaterService()

    private let storage = AppStorage()
    private let usageService = UsageService()
    private var refreshLoop: Task<Void, Never>?
    private var hasStarted = false

    init() {
        config = storage.loadConfig()
        snapshot = storage.loadSnapshot()
    }

    deinit {
        refreshLoop?.cancel()
    }

    var statusText: String {
        if !message.isEmpty {
            return message
        }
        guard let snapshot else { return "正在读取 Codex 用量" }
        switch snapshot.status {
        case .ok:
            return "用量状态正常"
        case .notLoggedIn:
            return "未检测到 Codex 登录状态"
        case .invalidAuth:
            return "Codex 凭据已失效"
        case .requestFailed:
            return "用量查询失败"
        }
    }

    var menuBarTitle: String {
        MenuBarPresentation.title(snapshot: snapshot, mode: config.menuBarDisplay)
    }

    var sourceText: String {
        switch snapshot?.source {
        case .appServer:
            "App Server"
        case .authJSON:
            "AuthJson"
        case .sessionLog:
            "Session Log"
        case nil:
            "加载中"
        }
    }

    var needsLogin: Bool {
        snapshot?.status == .notLoggedIn || snapshot?.status == .invalidAuth
    }

    func startIfNeeded() async {
        guard !hasStarted else { return }
        hasStarted = true
        updater.setAutomaticChecks(config.automaticallyChecksForUpdates)
        if config.startOnBoot {
            try? LoginItemService.setEnabled(true)
        }
        restartRefreshLoop()
    }

    func refresh() async {
        guard !isRefreshing else { return }
        isRefreshing = true
        message = ""
        let refreshed = await usageService.refresh(config: config)
        snapshot = mergeFailure(refreshed, previous: snapshot)
        isRefreshing = false

        if let snapshot {
            try? storage.saveSnapshot(snapshot)
        }
    }

    func binding<Value>(for keyPath: WritableKeyPath<AppConfig, Value>) -> Binding<Value> {
        Binding(
            get: { self.config[keyPath: keyPath] },
            set: { value in
                var updated = self.config
                updated[keyPath: keyPath] = value
                self.applyConfig(updated)
            }
        )
    }

    func setStartOnBoot(_ enabled: Bool) {
        do {
            try LoginItemService.setEnabled(enabled)
            var updated = config
            updated.startOnBoot = enabled
            applyConfig(updated)
        } catch {
            message = "无法更新登录时启动设置"
        }
    }

    func openCodexLogin() {
        guard let executable = CodexCommandResolver.resolve(config.command) else {
            message = "找不到 Codex 命令"
            return
        }
        let process = Process()
        process.executableURL = executable
        process.arguments = ["login"]
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
            message = "已打开 Codex 登录"
        } catch {
            message = "无法启动 Codex 登录"
        }
    }

    func quit() {
        NSApplication.shared.terminate(nil)
    }

    private func applyConfig(_ updated: AppConfig) {
        let refreshIntervalChanged = updated.refreshIntervalSeconds != config.refreshIntervalSeconds
        config = updated
        do {
            try storage.saveConfig(updated)
            message = ""
        } catch {
            message = "设置保存失败"
        }
        updater.setAutomaticChecks(updated.automaticallyChecksForUpdates)
        if refreshIntervalChanged {
            restartRefreshLoop()
        }
    }

    private func restartRefreshLoop() {
        refreshLoop?.cancel()
        refreshLoop = Task { [weak self] in
            guard let self else { return }
            await self.refresh()
            while !Task.isCancelled {
                let seconds = max(30, self.config.refreshIntervalSeconds)
                try? await Task.sleep(for: .seconds(seconds))
                guard !Task.isCancelled else { return }
                await self.refresh()
            }
        }
    }

    private func mergeFailure(
        _ refreshed: CodexUsageSnapshot,
        previous: CodexUsageSnapshot?
    ) -> CodexUsageSnapshot {
        guard refreshed.status != .ok,
              let previous,
              previous.status == .ok else {
            return refreshed
        }
        message = statusMessage(for: refreshed.status)
        return previous
    }

    private func statusMessage(for status: SnapshotStatus) -> String {
        switch status {
        case .ok:
            ""
        case .notLoggedIn:
            "未检测到 Codex 登录状态"
        case .invalidAuth:
            "Codex 凭据已失效"
        case .requestFailed:
            "刷新失败，显示上次结果"
        }
    }

}
