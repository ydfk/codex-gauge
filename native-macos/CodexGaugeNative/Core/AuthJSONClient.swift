import Foundation

public enum AuthJSONError: Error, Sendable {
    case notLoggedIn
    case invalidAuth
    case requestFailed

    public var snapshotStatus: SnapshotStatus {
        switch self {
        case .notLoggedIn:
            .notLoggedIn
        case .invalidAuth:
            .invalidAuth
        case .requestFailed:
            .requestFailed
        }
    }
}

public actor AuthJSONClient {
    private let usageURL = URL(string: "https://chatgpt.com/backend-api/wham/usage")!
    private let creditsURL = URL(
        string: "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits"
    )!

    public init() {}

    public func fetchSnapshot() async throws -> CodexUsageSnapshot {
        let auth = try readAuth()
        async let usageResult = requestJSON(usageURL, auth: auth)
        async let creditsResult = requestJSON(creditsURL, auth: auth)

        do {
            let usage = try await usageResult
            let credits = try? await creditsResult
            return UsageParser.parseWhamUsage(
                usage,
                credits: credits.flatMap(UsageParser.parseResetCredits),
                fallbackPlanType: auth.planType
            )
        } catch let error as AuthJSONError {
            _ = try? await creditsResult
            throw error
        } catch {
            throw AuthJSONError.requestFailed
        }
    }

    public func fetchCredits() async -> UsageCredits? {
        guard let auth = try? readAuth(),
              let value = try? await requestJSON(creditsURL, auth: auth) else {
            return nil
        }
        return UsageParser.parseResetCredits(value)
    }

    private func readAuth() throws -> AuthInfo {
        let path: URL
        if let configuredHome = ProcessInfo.processInfo.environment["CODEX_HOME"],
           !configuredHome.isEmpty {
            path = URL(fileURLWithPath: configuredHome).appendingPathComponent("auth.json")
        } else {
            path = FileManager.default.homeDirectoryForCurrentUser
                .appendingPathComponent(".codex/auth.json")
        }

        guard let data = try? Data(contentsOf: path),
              let value = try? JSONDecoder().decode(JSONValue.self, from: data),
              let accessToken = UsageParser.firstString(
                  in: value,
                  keys: ["access_token", "accessToken"]
              ),
              !accessToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw AuthJSONError.notLoggedIn
        }

        return AuthInfo(
            accessToken: accessToken,
            accountID: UsageParser.firstString(
                in: value,
                keys: ["account_id", "accountId", "chatgpt_account_id", "chatgptAccountId"]
            ),
            planType: UsageParser.firstString(
                in: value,
                keys: ["plan_type", "planType", "plan"]
            )
        )
    }

    private func requestJSON(_ url: URL, auth: AuthInfo) async throws -> JSONValue {
        var request = URLRequest(url: url)
        request.timeoutInterval = 8
        request.setValue("Bearer \(auth.accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("codex-1", forHTTPHeaderField: "OpenAI-Beta")
        request.setValue("Codex Desktop", forHTTPHeaderField: "originator")
        if let accountID = auth.accountID {
            request.setValue(accountID, forHTTPHeaderField: "ChatGPT-Account-ID")
        }

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await URLSession.shared.data(for: request)
        } catch {
            throw AuthJSONError.requestFailed
        }

        guard let httpResponse = response as? HTTPURLResponse else {
            throw AuthJSONError.requestFailed
        }
        if httpResponse.statusCode == 401 || httpResponse.statusCode == 403 {
            throw AuthJSONError.invalidAuth
        }
        guard 200 ..< 300 ~= httpResponse.statusCode,
              let value = try? JSONDecoder().decode(JSONValue.self, from: data) else {
            throw AuthJSONError.requestFailed
        }
        return value
    }
}

private struct AuthInfo: Sendable {
    let accessToken: String
    let accountID: String?
    let planType: String?
}

