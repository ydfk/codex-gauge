import Foundation

actor UsageService {
    private let authClient = AuthJSONClient()

    func refresh(config: AppConfig) async -> CodexUsageSnapshot {
        switch config.preferredProvider {
        case .appServer:
            if let appServerSnapshot = await readAppServer(command: config.command),
               appServerSnapshot.hasCompleteUsage {
                return appServerSnapshot
            }
            return await readAuthFallback()
        case .api:
            do {
                let snapshot = try await authClient.fetchSnapshot()
                if snapshot.hasCompleteUsage {
                    return snapshot
                }
            } catch let error as AuthJSONError {
                if let appServerSnapshot = await readAppServer(command: config.command),
                   appServerSnapshot.hasCompleteUsage {
                    return appServerSnapshot
                }
                return .empty(source: .authJSON, status: error.snapshotStatus)
            } catch {
                return .empty(source: .authJSON, status: .requestFailed)
            }

            return await readAppServer(command: config.command)
                ?? .empty(source: .authJSON, status: .requestFailed)
        }
    }

    private func readAuthFallback() async -> CodexUsageSnapshot {
        do {
            return try await authClient.fetchSnapshot()
        } catch let error as AuthJSONError {
            return .empty(source: .authJSON, status: error.snapshotStatus)
        } catch {
            return .empty(source: .authJSON, status: .requestFailed)
        }
    }

    private func readAppServer(command: String) async -> CodexUsageSnapshot? {
        do {
            let client = try AppServerClient(command: command)
            try client.initialize()
            let account = try? client.request("account/read")
            let rateLimits = try client.request("account/rateLimits/read")
            let credits = await authClient.fetchCredits()
            return UsageParser.parseAppServer(
                account: account,
                rateLimits: rateLimits,
                credits: credits
            )
        } catch {
            return nil
        }
    }
}

