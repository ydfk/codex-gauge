import Foundation

public enum CodexCommandResolver {
    public static func resolve(_ configuredCommand: String) -> URL? {
        let command = configuredCommand.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !command.isEmpty else { return nil }

        if command != "codex" {
            let expanded = NSString(string: command).expandingTildeInPath
            let url = URL(fileURLWithPath: expanded)
            return FileManager.default.isExecutableFile(atPath: url.path) ? url : nil
        }

        return candidates().first {
            FileManager.default.isExecutableFile(atPath: $0.path)
        }
    }

    private static func candidates() -> [URL] {
        var paths = [
            "/Applications/ChatGPT.app/Contents/Resources/codex",
            "/Applications/Codex.app/Contents/Resources/codex",
            "/opt/homebrew/bin/codex",
            "/usr/local/bin/codex",
        ]
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        paths.append(contentsOf: [
            "\(home)/.local/bin/codex",
            "\(home)/.volta/bin/codex",
            "\(home)/.npm-global/bin/codex",
        ])

        let pathDirectories = ProcessInfo.processInfo.environment["PATH"]?
            .split(separator: ":")
            .map(String.init) ?? []
        paths.append(contentsOf: pathDirectories.map { "\($0)/codex" })

        var seen = Set<String>()
        return paths
            .filter { seen.insert($0).inserted }
            .map { URL(fileURLWithPath: $0) }
    }
}

