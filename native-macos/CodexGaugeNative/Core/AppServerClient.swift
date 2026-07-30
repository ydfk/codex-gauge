import Darwin
import Foundation

public enum AppServerError: Error, Sendable {
    case commandNotFound
    case launchFailed
    case timedOut
    case invalidResponse
    case serverRejectedRequest
}

public final class AppServerClient {
    private let process: Process
    private let input: FileHandle
    private let output: FileHandle
    private var nextIdentifier = 1

    public init(command: String) throws {
        guard let executable = CodexCommandResolver.resolve(command) else {
            throw AppServerError.commandNotFound
        }

        let process = Process()
        let inputPipe = Pipe()
        let outputPipe = Pipe()
        process.executableURL = executable
        process.arguments = ["app-server"]
        process.standardInput = inputPipe
        process.standardOutput = outputPipe
        process.standardError = FileHandle.nullDevice

        do {
            try process.run()
        } catch {
            throw AppServerError.launchFailed
        }

        self.process = process
        input = inputPipe.fileHandleForWriting
        output = outputPipe.fileHandleForReading
    }

    deinit {
        input.closeFile()
        output.closeFile()
        if process.isRunning {
            process.terminate()
        }
    }

    public func initialize() throws {
        let identifier = takeIdentifier()
        try send([
            "jsonrpc": "2.0",
            "id": identifier,
            "method": "initialize",
            "params": [
                "clientInfo": [
                    "name": "codex-gauge-native",
                    "version": "0.1.0",
                ],
                "capabilities": [:],
                "protocolVersion": "2024-11-05",
            ],
        ])
        _ = try readResponse(identifier: identifier)
        try send([
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": [:],
        ])
    }

    public func request(_ method: String) throws -> JSONValue {
        let identifier = takeIdentifier()
        try send([
            "jsonrpc": "2.0",
            "id": identifier,
            "method": method,
        ])
        return try readResponse(identifier: identifier)
    }

    private func send(_ value: [String: Any]) throws {
        var data = try JSONSerialization.data(withJSONObject: value)
        data.append(0x0A)
        do {
            try input.write(contentsOf: data)
        } catch {
            throw AppServerError.launchFailed
        }
    }

    private func readResponse(identifier: Int) throws -> JSONValue {
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            let value = try readJSONLine(deadline: deadline)
            guard value["id"]?.int64Value == Int64(identifier) else { continue }
            if value["error"] != nil {
                throw AppServerError.serverRejectedRequest
            }
            return value
        }
        throw AppServerError.timedOut
    }

    private func readJSONLine(deadline: Date) throws -> JSONValue {
        var bytes: [UInt8] = []
        let descriptor = output.fileDescriptor

        while Date() < deadline {
            var descriptorState = pollfd(fd: descriptor, events: Int16(POLLIN), revents: 0)
            let remaining = max(1, Int32(deadline.timeIntervalSinceNow * 1_000))
            let result = Darwin.poll(&descriptorState, 1, remaining)
            if result == 0 {
                throw AppServerError.timedOut
            }
            if result < 0 {
                continue
            }

            var byte: UInt8 = 0
            let count = Darwin.read(descriptor, &byte, 1)
            guard count > 0 else { throw AppServerError.invalidResponse }
            if byte == 0x0A {
                guard !bytes.isEmpty else { continue }
                do {
                    return try JSONDecoder().decode(JSONValue.self, from: Data(bytes))
                } catch {
                    throw AppServerError.invalidResponse
                }
            }
            bytes.append(byte)
        }
        throw AppServerError.timedOut
    }

    private func takeIdentifier() -> Int {
        defer { nextIdentifier += 1 }
        return nextIdentifier
    }
}

