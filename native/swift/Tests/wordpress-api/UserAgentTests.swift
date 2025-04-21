import Foundation
import Testing
import WordPressAPI

@Suite("User Agent Tests")
struct UserAgentTests {
    #if canImport(Darwin)
    @Test("User agent contains bundle name and version", .enabled(if: isXCTest))
    func testThatDefaultUserAgentContainsBundleNameAndVersion() throws {
        #expect(UserAgent.postfix.contains(/xctest\/(\d+.\d+)/))
    }

    @Test("User agent contains CFNetwork version", .enabled(if: isXCTest))
    func testThatDefaultUserAgentContainsCFNetworkVersion() throws {
        #expect(UserAgent.postfix.contains(/CFNetwork\/(\d+.\d+.\d+)/))
    }

    @Test("User agent contains Darwin version")
    func testThatDefaultUserAgentContainsDarwinVersion() throws {
        #expect(UserAgent.postfix.contains(/Darwin\/(\d+.\d+.\d+)/))
    }

    @Test("User agent contains architecture")
    func testThatDefaultUserAgentContainsArchitecture() throws {
        #expect(UserAgent.postfix.contains("arm64"))
    }
    #elseif os(Linux)
    @Test("User agent contains architecture")
    func testThatUserAgentContainsArchitecture() throws {
        #expect(UserAgent.postfix.contains("aarch64") || UserAgent.postfix.contains("x86_64")) // CI is `x86_64`
    }
    #endif
}
