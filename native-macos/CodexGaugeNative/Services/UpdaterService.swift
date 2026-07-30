import Foundation
import Sparkle

@MainActor
final class UpdaterService {
    private let controller: SPUStandardUpdaterController
    let isConfigured: Bool

    init() {
        let publicKey = Bundle.main.object(
            forInfoDictionaryKey: "SUPublicEDKey"
        ) as? String ?? ""
        isConfigured = !publicKey.isEmpty && !publicKey.hasPrefix("REPLACE_")
        controller = SPUStandardUpdaterController(
            startingUpdater: isConfigured,
            updaterDelegate: nil,
            userDriverDelegate: nil
        )
    }

    func setAutomaticChecks(_ enabled: Bool) {
        guard isConfigured else { return }
        controller.updater.automaticallyChecksForUpdates = enabled
    }

    func checkForUpdates() {
        guard isConfigured else { return }
        controller.checkForUpdates(nil)
    }
}
