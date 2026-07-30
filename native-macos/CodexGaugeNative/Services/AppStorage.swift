import Foundation

struct AppStorage {
    private let fileManager = FileManager.default
    private let root: URL

    init() {
        let base = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first ?? fileManager.homeDirectoryForCurrentUser
        root = base.appendingPathComponent("Codex Gauge Native", isDirectory: true)
    }

    func loadConfig() -> AppConfig {
        read(AppConfig.self, from: root.appendingPathComponent("config.json")) ?? AppConfig()
    }

    func saveConfig(_ config: AppConfig) throws {
        try write(config, to: root.appendingPathComponent("config.json"))
    }

    func loadSnapshot() -> CodexUsageSnapshot? {
        read(CodexUsageSnapshot.self, from: root.appendingPathComponent("snapshot.json"))
    }

    func saveSnapshot(_ snapshot: CodexUsageSnapshot) throws {
        try write(snapshot, to: root.appendingPathComponent("snapshot.json"))
    }

    private func read<Value: Decodable>(_ type: Value.Type, from url: URL) -> Value? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return try? JSONDecoder().decode(type, from: data)
    }

    private func write<Value: Encodable>(_ value: Value, to url: URL) throws {
        try fileManager.createDirectory(at: root, withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(value)
        try data.write(to: url, options: .atomic)
    }
}

