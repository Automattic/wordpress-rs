import Foundation
import ArgumentParser
import WordPressAPI
import SwiftCSV

@main
struct WPCLITestingTool: AsyncParsableCommand {
//  @Option(help: "Specify the input")
//  public var input: String

    static let configuration = CommandConfiguration(
        abstract: "A utility for testing wordpress-rs in real-life scenarios",
        subcommands: [
            TestApplicationPasswordsEnabledCommand.self,
            AnalyzeTestResultsCommand.self,
        ]
    )

  public func run() throws {
      throw CleanExit.helpRequest(self)
  }
}

struct ApplicationPasswordTestResult: Codable {
    let siteUrl: URL
    let loginUrl: URL?
    let success: Bool
    let error: ApplicationPasswordTestResultError?
}

enum ApplicationPasswordTestResultError: String, ExpressibleByStringLiteral, Codable {

    case wordfence
    case hostingerTools
    case fluentAuth
    case multipleBadPlugins
    case unhelpfulError
    case unableToParse
    case unknownApplicationPasswordsDisabled
    case testFailure
    case invalidResponse
    case invalidssl
    case siteUnreachable
    case unknown

    init(stringLiteral value: StringLiteralType) {
        self = Self.parse(value)
    }

    private static func parse(_ string: String) -> ApplicationPasswordTestResultError {
        if string.contains("Wordfence") {
            return .wordfence
        }

        if string.contains("Hostinger") {
            return .hostingerTools
        }

        if string.contains("FluentAuth") {
            return .fluentAuth
        }

        if string.contains("there are multiple installed plugins that might have disabled Application Passwords") {
            return .multipleBadPlugins
        }

        if string.contains("Failed to parse api details") {
            return .unableToParse
        }

        if string.contains("Api root link header not found") {
            return .unhelpfulError
        }

        if string.contains("Application Passwords are not supported") {
            return .unknownApplicationPasswordsDisabled
        }

        if string.contains("Cannot allocate memory") {
            return .testFailure
        }

        if string.contains("The certificate for this server is invalid") || string.contains("An SSL error has occurred") {
            return .invalidssl
        }

        if string.contains("specified hostname could not be found")
            || string.contains("The request timed out")
            || string.contains("The network connection was lost")
        {
            return .siteUnreachable
        }

        if string.contains("returning invalid data") {
            return .invalidResponse
        }

        return .unknown
    }
}

struct TestApplicationPasswordsEnabledCommand: AsyncParsableCommand {

    private var session = URLSession(configuration: .default)

    private let maxConcurrency: Int = 20


    @Option(help: "Specify a single site URL")
    public var siteUrl: String?

    @Option(help: "Specify the CSV file you'd like to use")
    public var csvFilePath: String?

    @Option(help: "Specify the path for the output file")
    public var outputFilePath: String?

    static let configuration = CommandConfiguration(
        commandName: "test-application-passwords-enabled"
    )

    public func run() async throws {
        if let inputPath = self.csvFilePath, let outputPath = self.outputFilePath {
            try await self.handleCsvFile(inputPath: inputPath, outputPath: outputPath)
        }

        if let siteUrl {
            print("Testing 1 site...")
            let result = try await testSite(withUrl: URL(string: siteUrl)!, printingRawError: true)
            print(result)
        }
    }

    private func handleCsvFile(inputPath: String, outputPath: String) async throws {
        let csvFile: CSV = try CSV<Named>(url: URL(fileURLWithPath: inputPath))

        let urls: [URL] = csvFile.rows
            .compactMap { row in
                guard let string = row["URL"] else { return nil }
                return URL(string: string)
            }

        print("Testing \(urls.count) sites...")

        let results = try await withThrowingTaskGroup(of: ApplicationPasswordTestResult.self) { group in
            var results: [ApplicationPasswordTestResult] = []

            for item in urls.enumerated() {
                if item.offset >= maxConcurrency {
                    if let result = try await group.next() {
                        results.append(result)
                        if let error = result.error {
                            print(error.rawValue)
                        }
                    }
                }
                group.addTask {
                    try await testSite(withUrl: item.element)
                }

            }

            try await group.waitForAll()

            return results
        }

        let data = try JSONEncoder().encode(results)
        FileManager.default.createFile(atPath: outputPath, contents: data)
    }

    private func testSite(withUrl url: URL, printingRawError: Bool = false) async throws -> ApplicationPasswordTestResult {
        do {
            let apiClient = WordPressLoginClient(urlSession: session)
            let loginUrl = try await apiClient.loginURL(forSite: url.absoluteString)
            return ApplicationPasswordTestResult(
                siteUrl: url,
                loginUrl: loginUrl.asURL(),
                success: true,
                error: nil
            )
        } catch {
            if printingRawError {
                debugPrint(error)
                print(error.localizedDescription)
            }

            return ApplicationPasswordTestResult(
                siteUrl: url,
                loginUrl: nil,
                success: false,
                error: ApplicationPasswordTestResultError(stringLiteral: error.localizedDescription)
            )
        }
    }

    enum CodingKeys: CodingKey {
        case csvFilePath
        case outputFilePath
        case siteUrl
    }
}

struct AnalyzeTestResultsCommand: AsyncParsableCommand {
    static let configuration = CommandConfiguration(commandName: "analyze-test-results")

    @Argument(help: "Specify the CSV file you'd like to use")
    public var resultFilePath: String


    public func run() async throws {
        let data = try Data(contentsOf: URL(fileURLWithPath: resultFilePath))
        let rawData = try JSONDecoder().decode([ApplicationPasswordTestResult].self, from: data)

        let total = rawData.count
        var success: Int = 0
        var errorMessages: [String: Int] = [:]

        for result in rawData {
            if result.success {
                success += 1
            }

            if let errorMessage = result.error {
                if errorMessages[errorMessage.rawValue] != nil {
                    errorMessages[errorMessage.rawValue]! += 1
                } else {
                    errorMessages[errorMessage.rawValue] = 1
                }
            }
        }

        print("====== Results ====== ")
        print("Total Entries: \(total)")
        print("Success: \(success)")
        print("Failure: \(total - success)")
        print("Sucess Rate: \(Double(success) / Double(total) * 100)")
        print("Error Rates:")

        for error in errorMessages.sorted(by: { $0.key > $1.key }) {
            print("\t\(error.key)\t\t\(error.value)")
        }
    }
}
