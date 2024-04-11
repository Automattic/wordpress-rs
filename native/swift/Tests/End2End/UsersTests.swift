#if os(macOS) || os(Linux)

import XCTest
import wordpress_api

class UsersTests: XCTestCase {

    func testGetCurrentUser() async throws {
        let user = try await site.api.users.view.getCurrent()
        XCTAssertEqual(user.id, site.currentUserID)
    }

    func testGetUser() async throws {
        let user = try await site.api.users.view.get(id: 2)
        XCTAssertEqual(user.name, "Theme Buster")
    }

    func testDeleteCurrent() async throws {
        throw XCTSkip("Need to create a user with an application password for this test to work")

        let password = "supersecurepassword"
        let newUser = try await site.createUser(password: password)
        let newUserSession = WordPressAPI(
            urlSession: .shared, baseUrl: site.siteURL,
            authenticationStategy: .init(username: newUser.username, password: password))

        let user = try await newUserSession.users.view.getCurrent()
        XCTAssertEqual(user.id, newUser.id)
        try await newUserSession.users.deleteCurrent(reassignTo: site.currentUserID)

        do {
            // Should return 404
            _ = try await site.api.users.view.get(id: newUser.id)
            XCTFail("Unexpected successful result. The user \(newUser.id) should have been deleted.")
        } catch {
            // Do nothing
        }
    }

    func testCreateAndDeleteUser() async throws {
        let newUser = try await site.createUser()
        try await site.api.users.delete(id: newUser.id, reassignTo: site.currentUserID)
    }

    func testUpdateCurrentUser() async throws {
        let currentUser = try await site.api.users.view.getCurrent()
        let newDescription = currentUser.description + " and more"
        let updated = try await site.api.users.updateCurrent(
            with: .init(
                name: nil, firstName: nil, lastName: nil, email: nil, url: nil,
                description: newDescription, locale: nil, nickname: nil, slug: nil, roles: [],
                password: nil, meta: nil))
        XCTAssertEqual(updated.description, newDescription)
    }

    func testPatchUpdate() async throws {
        let newUser = try await site.createUser()

        let firstUpdate = try await site.api.users.update(
            id: newUser.id,
            with: .init(
                name: nil, firstName: "Adam", lastName: nil, email: nil, url: "https://newurl.com",
                description: nil, locale: nil, nickname: nil, slug: nil, roles: [], password: nil,
                meta: nil))
        XCTAssertEqual(firstUpdate.firstName, "Adam")
        XCTAssertEqual(firstUpdate.url, "https://newurl.com")

        let secondUpdate = try await site.api.users.update(
            id: newUser.id,
            with: .init(
                name: nil, firstName: nil, lastName: nil, email: nil, url: "https://w.org",
                description: nil, locale: nil, nickname: nil, slug: nil, roles: [], password: nil,
                meta: nil))
        XCTAssertEqual(secondUpdate.firstName, "Adam")
        XCTAssertEqual(secondUpdate.url, "https://w.org")
    }

    func testListUsers() async throws {
        let users = try await site.api.users.view.list()
        XCTAssertTrue(users.count > 0)
    }
}

class UserCreationErrorTests: XCTestCase {

    func testUsernameAlreadyExists() async throws {
        let uuid = UUID().uuidString
        _ = try await site.api.users.create(
            using: .init(
                username: uuid, email: "\(uuid)@test.com", password: "badpass", name: nil, firstName: nil,
                lastName: nil, url: nil, description: nil, locale: nil, nickname: nil, slug: nil,
                roles: ["subscriber"], meta: nil))

        let error = await assertThrow {
            _ = try await site.api.users.create(
                using: .init(
                    username: uuid, email: "\(UUID().uuidString)@test.com", password: "badpass", name: nil,
                    firstName: nil, lastName: nil, url: nil, description: nil, locale: nil, nickname: nil,
                    slug: nil, roles: ["subscriber"], meta: nil))
        }

        let apiError = try XCTUnwrap(error as? WpApiError, "Error is not `WpApiError` type")
        switch apiError {
        case let .ServerError(statusCode):
            XCTAssertEqual(statusCode, 500)
        default:
            XCTFail("Unexpected error: \(apiError)")
        }
    }

    func testIllegalEmail() async throws {
        let error = await assertThrow {
            _ = try await site.api.users.create(
                using: .init(
                    username: "\(UUID().uuidString)", email: "test.com", password: "badpass", name: nil,
                    firstName: nil, lastName: nil, url: nil, description: nil, locale: nil, nickname: nil,
                    slug: nil, roles: ["subscriber"], meta: nil))
        }

        let apiError = try XCTUnwrap(error as? WpApiError, "Error is not `WpApiError` type")
        switch apiError {
        case let .ClientError(_, statusCode):
            XCTAssertEqual(statusCode, 400)
        default:
            XCTFail("Unexpected error: \(apiError)")
        }
    }

    func testIllegalRole() async throws {
        let error = await assertThrow {
            let uuid = UUID().uuidString
            _ = try await site.api.users.create(
                using: .init(
                    username: uuid, email: "\(uuid)@test.com", password: "badpass", name: nil,
                    firstName: nil, lastName: nil, url: nil, description: nil, locale: nil, nickname: nil,
                    slug: nil, roles: ["sub"], meta: nil))
        }

        let apiError = try XCTUnwrap(error as? WpApiError, "Error is not `WpApiError` type")
        switch apiError {
        case let .ClientError(_, statusCode):
            XCTAssertEqual(statusCode, 400)
        default:
            XCTFail("Unexpected error: \(apiError)")
        }
    }

    private func assertThrow(
        closure: () async throws -> Void, file: StaticString = #file, line: UInt = #line
    ) async -> Error {
        do {
            try await closure()
            XCTFail("Expect an error shown in the above call", file: file, line: line)
            throw NSError(domain: "assert-throw", code: 1)
        } catch {
            return error
        }
    }

}

class UserContextTests: XCTestCase {

    func testGetCurrent() async throws {
        let users = try await site.api.users
        let view = try await users.view.getCurrent()
        let edit = try await users.edit.getCurrent()
        let embed = try await users.embed.getCurrent()

        XCTAssertEqual(view.id, edit.id)
        XCTAssertEqual(edit.id, embed.id)

        XCTAssertEqual(view.name, edit.name)
        XCTAssertEqual(edit.name, embed.name)

        XCTAssertNotNil(edit.email)
    }

    func testGetUser() async throws {
        let newUser = try await site.createUser()
        addTeardownBlock {
            try await site.api.users.delete(id: newUser.id, reassignTo: site.currentUserID)
        }

        let users = try await site.api.users
        let view = try await users.view.get(id: newUser.id)
        let edit = try await users.edit.get(id: newUser.id)
        let embed = try await users.embed.get(id: newUser.id)

        XCTAssertEqual(view.id, edit.id)
        XCTAssertEqual(edit.id, embed.id)

        XCTAssertEqual(view.name, edit.name)
        XCTAssertEqual(edit.name, embed.name)

        XCTAssertNotNil(edit.email)
    }

    func testEditContext() async throws {
        let edit = try await site.api.users.edit.getCurrent()
        XCTAssertEqual(edit.roles, ["administrator"])
        XCTAssertNotNil(edit.locale)
        XCTAssertTrue(edit.capabilities.count > 0)
    }

    func testList() async throws {
        let users = try await site.api.users
        let view = try await users.view.list().first
        let edit = try await users.edit.list().first
        let embed = try await users.embed.list().first

        XCTAssertNotNil(view)
        XCTAssertNotNil(edit)
        XCTAssertNotNil(embed)

        XCTAssertEqual(view?.id, edit?.id)
        XCTAssertEqual(edit?.id, embed?.id)

        XCTAssertEqual(view?.name, edit?.name)
        XCTAssertEqual(edit?.name, embed?.name)

        XCTAssertNotNil(edit?.email)
    }

}

#endif
