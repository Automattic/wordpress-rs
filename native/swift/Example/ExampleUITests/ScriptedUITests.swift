import XCTest

/// Drives the example app from a script of steps, and saves a screenshot and a list of what's on screen wherever
/// the script asks for one. It's how the Reader screens are checked on a simulator without tapping by hand.
///
/// Run it with `scripts/example-app-ui-script.sh`, which passes these to the test:
///
/// - `UI_SCRIPT`: the script's path.
/// - `UI_OUTPUT`: the folder for screenshots, screen lists and the transcript.
///
/// A script has one step a line. Blank lines and lines starting with `//` are skipped. An element is named by its
/// label, by `#` and its accessibility identifier, or by `~` and part of its label. A number and `:` in front
/// picks which match, counting from 0: `2:#stream-post`.
///
///     launch [arguments]        Starts the app again, with launch arguments.
///     activate                  Brings the app forward without restarting it.
///     tap <element>             Scrolls down until the element can be tapped, and taps it.
///     tapnow <element>          Taps the element where it is, without scrolling.
///     type <element> => <text>  Taps a text field and types into it. `\n` presses return.
///     clear <element>           Empties a text field.
///     alert <button>            Taps a button of the alert that's showing.
///     back                      Goes back a screen.
///     up [n] / down [n]         Scrolls up or down the list, n times (once by default).
///     top                       Scrolls to the top of the list.
///     bottom [n]                Flicks towards the end of the list, n times (8 by default).
///     pull                      Pulls down to refresh.
///     wait <seconds>            Waits.
///     waitfor <element> [s]     Waits until the element exists, for 20 seconds unless told otherwise.
///     waitgone <element> [s]    Waits until the element is gone.
///     expect <element>          Fails the test unless the element exists.
///     expectnot <element>       Fails the test if the element exists.
///     shot <name>               Saves `<name>.png` and `<name>.txt`: the screen, and what's on it.
final class ScriptedUITests: XCTestCase {

    @MainActor
    func testRunScript() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let scriptPath = environment["UI_SCRIPT"], let outputPath = environment["UI_OUTPUT"] else {
            throw XCTSkip("Run this with scripts/example-app-ui-script.sh, which says which script to run.")
        }
        let output = URL(fileURLWithPath: outputPath, isDirectory: true)
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        let script = try String(contentsOfFile: scriptPath, encoding: .utf8)

        continueAfterFailure = true
        var runner = ScriptRunner(output: output)

        for (index, line) in script.components(separatedBy: .newlines).enumerated() {
            let step = line.trimmingCharacters(in: .whitespaces)
            guard !step.isEmpty, !step.hasPrefix("//") else { continue }
            do {
                try runner.run(step)
                try runner.note("ok    \(index + 1): \(step)")
            } catch {
                try runner.note("FAIL  \(index + 1): \(step) — \(error)")
                XCTFail("Line \(index + 1) (\(step)) failed: \(error)")
                do {
                    try runner.shot("failure-line-\(index + 1)")
                } catch {
                    XCTFail("The screen at the failure couldn't be saved: \(error)")
                }
                // What follows a step that didn't happen would be about another screen.
                break
            }
        }
    }
}

private struct ScriptError: Error, CustomStringConvertible {
    let description: String
}

@MainActor
private struct ScriptRunner {
    let app = XCUIApplication()
    let output: URL
    private var transcript: [String] = []

    init(output: URL) {
        self.output = output
    }

    /// Adds a line to the transcript, and saves it: a step that stops the test mustn't lose what came before.
    mutating func note(_ line: String) throws {
        transcript.append(line)
        let text = transcript.joined(separator: "\n") + "\n"
        try text.write(to: output.appendingPathComponent("transcript.txt"), atomically: true, encoding: .utf8)
    }

    // swiftlint:disable:next cyclomatic_complexity
    mutating func run(_ step: String) throws {
        let (command, argument) = Self.split(step)
        switch command {
        case "launch":
            app.launchArguments = argument.split(separator: " ").map(String.init)
            app.launch()
        case "activate": app.activate()
        case "tap": tap(try element(argument, scrolls: true))
        case "tapnow": tap(try element(argument, scrolls: false))
        case "type": try type(argument)
        case "clear": try clear(argument)
        case "alert": try tapAlertButton(argument)
        case "back": try goBack()
        case "up": repeated(argument) { scroll(from: 0.35, to: 0.75) }
        case "down": repeated(argument) { scroll(from: 0.75, to: 0.35) }
        case "top": toTop()
        case "bottom": repeated(argument.isEmpty ? "8" : argument) { app.windows.firstMatch.swipeUp(velocity: .fast) }
        case "pull": scroll(from: 0.3, to: 0.9)
        case "wait": Thread.sleep(forTimeInterval: Double(argument) ?? 1)
        case "waitfor": try wait(argument, exists: true)
        case "waitgone": try wait(argument, exists: false)
        case "expect":
            XCTAssertTrue(matches(argument).firstMatch.exists, "Expected \(argument) on screen")
        case "expectnot":
            XCTAssertFalse(matches(argument).firstMatch.exists, "Expected \(argument) not to be on screen")
        case "shot": try shot(argument)
        default: throw ScriptError(description: "Unknown step '\(command)'")
        }
    }

    // MARK: Steps

    func shot(_ name: String) throws {
        let screenshot = XCUIScreen.main.screenshot().pngRepresentation
        try screenshot.write(to: output.appendingPathComponent("\(name).png"))
        let screen = Self.describe(try app.snapshot(), depth: 0)
        try screen.write(to: output.appendingPathComponent("\(name).txt"), atomically: true, encoding: .utf8)
    }

    private func type(_ argument: String) throws {
        let parts = argument.components(separatedBy: " => ")
        guard parts.count == 2 else {
            throw ScriptError(description: "Expected '<element> => <text>'")
        }
        let field = try element(parts[0], scrolls: true)
        field.tap()
        field.typeText(parts[1].replacingOccurrences(of: "\\n", with: "\n"))
    }

    private func clear(_ selector: String) throws {
        let field = try element(selector, scrolls: true)
        field.tap()
        let text = field.value as? String ?? ""
        field.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: text.count))
    }

    private func tap(_ element: XCUIElement) {
        if element.elementType == .switch {
            // A switch's row doesn't flip it: the switch at its end does.
            element.coordinate(withNormalizedOffset: CGVector(dx: 0.92, dy: 0.5)).tap()
        } else {
            element.tap()
        }
    }

    private func goBack() throws {
        let button = app.navigationBars.buttons.element(boundBy: 0)
        guard button.exists else {
            throw ScriptError(description: "There's no screen to go back from")
        }
        button.tap()
    }

    private func tapAlertButton(_ label: String) throws {
        let button = app.alerts.buttons[label]
        guard button.waitForExistence(timeout: 10) else {
            throw ScriptError(description: "No alert with a '\(label)' button")
        }
        button.tap()
    }

    private func wait(_ argument: String, exists: Bool) throws {
        var selector = argument
        var timeout = 20.0
        if let space = argument.lastIndex(of: " "), let seconds = Double(argument[argument.index(after: space)...]) {
            selector = String(argument[..<space])
            timeout = seconds
        }
        let deadline = Date().addingTimeInterval(timeout)
        while matches(selector).firstMatch.exists != exists {
            guard Date() < deadline else {
                throw ScriptError(description: "\(selector) \(exists ? "didn't appear" : "didn't go") in \(timeout)s")
            }
            Thread.sleep(forTimeInterval: 0.3)
        }
    }

    private func repeated(_ argument: String, _ work: () -> Void) {
        for _ in 0..<(Int(argument) ?? 1) {
            work()
        }
    }

    /// Drags the list between two heights of the screen, each a fraction from the top.
    private func scroll(from start: CGFloat, to end: CGFloat) {
        let window = app.windows.firstMatch
        let origin = window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: start))
        let destination = window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: end))
        origin.press(forDuration: 0.05, thenDragTo: destination)
    }

    private func toTop() {
        // Tapping the status bar scrolls a list to its top. The clock, not the middle: that's the Dynamic Island.
        app.windows.firstMatch.coordinate(withNormalizedOffset: CGVector(dx: 0.2, dy: 0.03)).tap()
        Thread.sleep(forTimeInterval: 0.8)
    }

    // MARK: Finding elements

    /// The element a selector names, scrolling down to it when `scrolls` is on. A list only has the rows near the
    /// screen, so a row further down isn't found until the list has been scrolled to it.
    private func element(_ selector: String, scrolls: Bool) throws -> XCUIElement {
        var (index, name) = (0, selector)
        if let colon = selector.firstIndex(of: ":"), let number = Int(selector[..<colon]) {
            index = number
            name = String(selector[selector.index(after: colon)...])
        }
        // A page of 20 long posts is more than a dozen drags from its end.
        for attempt in 0..<(scrolls ? 40 : 1) {
            if attempt > 0 {
                // A drag that starts on the keyboard doesn't move the list, so start above it.
                if app.keyboards.firstMatch.exists {
                    scroll(from: 0.45, to: 0.25)
                } else {
                    scroll(from: 0.7, to: 0.4)
                }
            }
            if let found = hittable(name, index: index, anywhere: !scrolls) {
                return found
            }
        }
        throw ScriptError(description: "Nothing to tap matches '\(selector)'")
    }

    /// The match to act on, counting from the top of the screen: a control before the text or the row that shares
    /// its label. Unless `anywhere` is on, a row under the navigation bar or the tab bar doesn't count: a tap there
    /// lands on the bar.
    private func hittable(_ name: String, index: Int, anywhere: Bool) -> XCUIElement? {
        let predicate = Self.predicate(name)
        if index == 0 {
            // A button of the navigation bar or the tab bar is where it always is.
            for bar in [app.navigationBars, app.tabBars] {
                let button = bar.buttons.matching(predicate).firstMatch
                if button.exists, button.isHittable {
                    return button
                }
            }
        }
        let screen = app.windows.firstMatch.frame
        let rows = (screen.minY + 110)...(screen.maxY - 110)
        let preferred: [XCUIElement.ElementType] = [
            .button, .link, .switch, .textField, .secureTextField, .textView, .menuItem, .staticText, .cell, .any
        ]
        for type in preferred {
            let found = app.descendants(matching: type).matching(predicate).allElementsBoundByIndex
                .filter {
                    $0.exists && $0.isHittable && (anywhere || rows.contains($0.frame.midY))
                }
            if found.count > index {
                return found[index]
            }
        }
        return nil
    }

    private func matches(_ selector: String) -> XCUIElementQuery {
        app.descendants(matching: .any).matching(Self.predicate(selector))
    }

    private static func predicate(_ selector: String) -> NSPredicate {
        if selector.hasPrefix("#") {
            return NSPredicate(format: "identifier == %@", String(selector.dropFirst()))
        }
        if selector.hasPrefix("~") {
            return NSPredicate(format: "label CONTAINS %@", String(selector.dropFirst()))
        }
        return NSPredicate(format: "label == %@", selector)
    }

    private static func split(_ step: String) -> (command: String, argument: String) {
        guard let space = step.firstIndex(of: " ") else {
            return (step, "")
        }
        return (String(step[..<space]), String(step[step.index(after: space)...]).trimmingCharacters(in: .whitespaces))
    }

    /// What's on screen, as text: every element that says something, indented by how deep it is.
    private static func describe(_ element: any XCUIElementSnapshot, depth: Int) -> String {
        var lines: [String] = []
        let value = (element.value as? String).flatMap { $0.isEmpty ? nil : $0 }
        let placeholder = element.placeholderValue.flatMap { $0.isEmpty ? nil : $0 }
        if !element.label.isEmpty || !element.identifier.isEmpty || value != nil || placeholder != nil {
            var line = String(repeating: "  ", count: min(depth, 12)) + name(of: element.elementType)
            if !element.label.isEmpty {
                line += " \"\(element.label)\""
            }
            if !element.identifier.isEmpty {
                line += " #\(element.identifier)"
            }
            if let value {
                line += " = \"\(value)\""
            }
            if let placeholder, placeholder != value {
                line += " placeholder \"\(placeholder)\""
            }
            if !element.isEnabled {
                line += " (disabled)"
            }
            line += " @\(Int(element.frame.midX)),\(Int(element.frame.midY))"
            lines.append(line)
        }
        for child in element.children {
            let text = describe(child, depth: depth + 1)
            if !text.isEmpty {
                lines.append(text)
            }
        }
        return lines.joined(separator: "\n")
    }

    // swiftlint:disable:next cyclomatic_complexity
    private static func name(of type: XCUIElement.ElementType) -> String {
        switch type {
        case .button: "button"
        case .staticText: "text"
        case .cell: "cell"
        case .textField: "field"
        case .textView: "textview"
        case .switch: "switch"
        case .link: "link"
        case .image: "image"
        case .navigationBar: "navbar"
        case .tabBar: "tabbar"
        case .alert: "alert"
        case .menuItem: "menuitem"
        case .activityIndicator: "spinner"
        case .webView: "webview"
        default: "other"
        }
    }
}
