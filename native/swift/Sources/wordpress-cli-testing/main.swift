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
    case unhelpfulError
    case unableToParse
    case unknownApplicationPasswordsDisabled
    case testFailure
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

        return .unknown
    }
}

struct TestApplicationPasswordsEnabledCommand: AsyncParsableCommand {

    private var session = URLSession(configuration: .default)

    private let maxConcurrency: Int = 100

    @Argument(help: "Specify the CSV file you'd like to use")
    public var csvFilePath: String

    @Argument(help: "Specify the path for the output file")
    public var outputFilePath: String

    static let configuration = CommandConfiguration(
        commandName: "test-application-passwords-enabled"
    )

    public func run() async throws {
        let csvFile: CSV = try CSV<Named>(url: URL(fileURLWithPath: csvFilePath))

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
                    do {
                        let apiClient = WordPressLoginClient(urlSession: session)
                        let loginUrl = try await apiClient.loginURL(forSite: item.element.absoluteString)
                        return ApplicationPasswordTestResult(
                            siteUrl: item.element,
                            loginUrl: loginUrl.asURL(),
                            success: true,
                            error: nil
                        )
                    } catch {
                        return ApplicationPasswordTestResult(
                            siteUrl: item.element,
                            loginUrl: nil,
                            success: false,
                            error: ApplicationPasswordTestResultError(stringLiteral: error.localizedDescription)
                        )
                    }
                }

            }

            try await group.waitForAll()

            return results
        }

        let data = try JSONEncoder().encode(results)
        FileManager.default.createFile(atPath: self.outputFilePath, contents: data)
    }

    enum CodingKeys: CodingKey {
        case csvFilePath
        case outputFilePath
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
