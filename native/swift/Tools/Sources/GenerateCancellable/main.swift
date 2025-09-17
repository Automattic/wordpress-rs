import ArgumentParser
import Foundation
import Logging
import SwiftSyntax
import SwiftParser
import SwiftSyntaxBuilder

@main
struct GenerateCancellable: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "generate-cancellable",
        abstract: "Generate cancellable APIs"
    )

    @Argument(help: "The Swift source file to analyze")
    var inputFile: String

    @Argument(help: "Output file path for generated extensions")
    var output: String

    @Option(name: .long, help: "Set log level (trace, debug, info, notice, warning, error, critical)")
    var logLevel: String = "info"

    mutating func run() throws {
        let logger = setupLogger()

        let inputURL = URL(fileURLWithPath: inputFile)
        let outputURL = URL(fileURLWithPath: output)

        logger.info("Analyzing file: \(inputURL.path)")
        logger.info("Generating extensions to: \(outputURL.path)")

        guard FileManager.default.fileExists(atPath: inputURL.path) else {
            throw ValidationError("Input file does not exist: \(inputURL.path)")
        }

        let sourceCode = try String(contentsOf: inputURL, encoding: .utf8)
        let extensionsCode = try generateExtensions(sourceCode: sourceCode, logger: logger)

        try extensionsCode.write(to: outputURL, atomically: true, encoding: .utf8)

        logger.info("Successfully generated extensions to: \(outputURL.path)")
    }

    private func setupLogger() -> Logger {
        let logLevelValue = Logger.Level(rawValue: logLevel.lowercased()) ?? .info

        LoggingSystem.bootstrap { label in
            var handler = StreamLogHandler.standardOutput(label: label)
            handler.logLevel = logLevelValue
            return handler
        }

        return Logger(label: "generate-cancellable")
    }

    private func generateExtensions(sourceCode: String, logger: Logger) throws -> String {
        let syntaxTree = Parser.parse(source: sourceCode)

        logger.debug("Parsed syntax tree successfully")

        let analyzer = CancellationAnalyzer(logger: logger)
        let analysis = analyzer.analyze(syntaxTree)

        logger.info("Found \(analysis.count) RequestExecutor classes")

        let generator = ExtensionGenerator(analysis: analysis, logger: logger)
        return generator.generateExtensionsCode()
    }
}

struct FunctionInfo {
    let declaration: FunctionDeclSyntax
    let name: String
    let parameters: FunctionParameterListSyntax
    let returnType: TypeSyntax?
    let modifiers: DeclModifierListSyntax?
    let attributes: AttributeListSyntax?
    let asyncKeyword: TokenSyntax?
    let throwsClause: ThrowsClauseSyntax?
}

struct ClassAnalysis {
    let className: String
    let cancellationFunctions: [FunctionInfo]
    let existingFunctions: [String: FunctionInfo]
}

class CancellationAnalyzer: SyntaxVisitor {
    private let logger: Logger
    private var analysis: [String: ClassAnalysis] = [:]
    private var currentClass: String?
    private var currentCancellationFunctions: [FunctionInfo] = []
    private var currentExistingFunctions: [String: FunctionInfo] = [:]

    init(logger: Logger) {
        self.logger = logger
        super.init(viewMode: .sourceAccurate)
    }

    func analyze(_ tree: SourceFileSyntax) -> [String: ClassAnalysis] {
        walk(tree)
        return analysis
    }

    override func visit(_ node: ClassDeclSyntax) -> SyntaxVisitorContinueKind {
        let className = node.name.text

        if className.hasSuffix("RequestExecutor") {
            logger.debug("Analyzing RequestExecutor class: \(className)")

            currentClass = className
            currentCancellationFunctions = []
            currentExistingFunctions = [:]

            return .visitChildren
        }

        return .skipChildren
    }

    override func visitPost(_ node: ClassDeclSyntax) {
        if let className = currentClass {
            analysis[className] = ClassAnalysis(
                className: className,
                cancellationFunctions: currentCancellationFunctions,
                existingFunctions: currentExistingFunctions
            )

            logger.debug("Found \(currentCancellationFunctions.count) cancellation functions in \(className)")
            logger.debug("Found \(currentExistingFunctions.count) existing functions in \(className)")

            currentClass = nil
        }
    }

    override func visit(_ node: FunctionDeclSyntax) -> SyntaxVisitorContinueKind {
        guard currentClass != nil else { return .skipChildren }

        let functionName = node.name.text

        let functionInfo = FunctionInfo(
            declaration: node,
            name: functionName,
            parameters: node.signature.parameterClause.parameters,
            returnType: node.signature.returnClause?.type,
            modifiers: node.modifiers,
            attributes: node.attributes,
            asyncKeyword: node.signature.effectSpecifiers?.asyncSpecifier,
            throwsClause: node.signature.effectSpecifiers?.throwsClause
        )

        if functionName.hasSuffix("Cancellation") && hasCancellationTokenParameter(node) {
            currentCancellationFunctions.append(functionInfo)
            logger.trace("Found cancellation function: \(functionName)")
        } else {
            currentExistingFunctions[functionName] = functionInfo
        }

        return .skipChildren
    }

    private func hasCancellationTokenParameter(_ function: FunctionDeclSyntax) -> Bool {
        let parameters = function.signature.parameterClause.parameters
        guard let lastParam = parameters.last else { return false }

        let paramName = lastParam.firstName.text
        let paramType = lastParam.type.description.trimmingCharacters(in: .whitespacesAndNewlines)

        return paramName == "cancellationToken" && paramType == "CancellationToken?"
    }
}

class ExtensionGenerator {
    private let analysis: [String: ClassAnalysis]
    private let logger: Logger

    init(analysis: [String: ClassAnalysis], logger: Logger) {
        self.analysis = analysis
        self.logger = logger
    }

    func generateExtensionsCode() -> String {
        var extensions: [ExtensionDeclSyntax] = []

        for (className, classAnalysis) in analysis {
            logger.debug("Generating extension for class: \(className)")

            let extensionMembers = generateExtensionMembers(for: classAnalysis)

            if !extensionMembers.isEmpty {
                let extensionDecl = ExtensionDeclSyntax(
                    extensionKeyword: .keyword(.extension, trailingTrivia: .space),
                    extendedType: IdentifierTypeSyntax(name: .identifier(className)),
                    memberBlock: MemberBlockSyntax(
                        leftBrace: .leftBraceToken(leadingTrivia: .space, trailingTrivia: .newlines(2)),
                        members: MemberBlockItemListSyntax(extensionMembers),
                        rightBrace: .rightBraceToken(leadingTrivia: .newline)
                    ),
                    trailingTrivia: .newlines(2)
                )
                extensions.append(extensionDecl)
                logger.info("Generated extension for \(className) with \(extensionMembers.count) functions")
            }
        }

        let sourceFile = SourceFileSyntax(
            statements: CodeBlockItemListSyntax(
                extensions.map { ext in
                    CodeBlockItemSyntax(item: .decl(DeclSyntax(ext)))
                }
            )
        )

        return """
        // Do not modify. This file is automatically generated.
        // swiftlint:disable all

        import Foundation

        \(sourceFile.description)
        """
    }

    private func generateExtensionMembers(for classAnalysis: ClassAnalysis) -> [MemberBlockItemSyntax] {
        var members: [MemberBlockItemSyntax] = []

        for cancellationFunc in classAnalysis.cancellationFunctions {
            let nonCancellationName = String(cancellationFunc.name.dropLast("Cancellation".count))

            if classAnalysis.existingFunctions[nonCancellationName] != nil {
                logger.warning("Skipping generation of '\(nonCancellationName)' - function already exists in \(classAnalysis.className)")
                continue
            }

            logger.debug("Generating new function: \(nonCancellationName)")

            let newFunction = createNonCancellationFunction(from: cancellationFunc, name: nonCancellationName)
            let memberItem = MemberBlockItemSyntax(
                decl: DeclSyntax(newFunction),
                trailingTrivia: .newline
            )
            members.append(memberItem)
        }

        return members
    }

    private func createNonCancellationFunction(from cancellationFunc: FunctionInfo, name: String) -> FunctionDeclSyntax {
        var parametersWithoutCancellation: FunctionParameterListSyntax

        if cancellationFunc.parameters.isEmpty || cancellationFunc.parameters.count == 1 {
            parametersWithoutCancellation = FunctionParameterListSyntax([])
        } else {
            var paramsArray = Array(cancellationFunc.parameters.dropLast())

            if !paramsArray.isEmpty {
                let lastIndex = paramsArray.count - 1
                paramsArray[lastIndex] = paramsArray[lastIndex].with(\.trailingComma, nil)
            }

            parametersWithoutCancellation = FunctionParameterListSyntax(paramsArray)
        }

        let effectSpecifiers = FunctionEffectSpecifiersSyntax(
            asyncSpecifier: cancellationFunc.asyncKeyword?.with(\.leadingTrivia, .space).with(\.trailingTrivia, .space),
            throwsClause: cancellationFunc.throwsClause?.with(\.leadingTrivia, .init()).with(\.trailingTrivia, .init())
        )

        let returnClause = cancellationFunc.returnType.map { type in
            let cleanType = type.with(\.leadingTrivia, .init()).with(\.trailingTrivia, .init())
            return ReturnClauseSyntax(
                arrow: .arrowToken(leadingTrivia: .space, trailingTrivia: .space),
                type: cleanType
            )
        }

        let signature = FunctionSignatureSyntax(
            parameterClause: FunctionParameterClauseSyntax(
                leftParen: .leftParenToken(),
                parameters: parametersWithoutCancellation,
                rightParen: .rightParenToken()
            ),
            effectSpecifiers: effectSpecifiers,
            returnClause: returnClause
        )

        let functionCallBody = createFunctionCallBody(
            cancellationFunctionName: cancellationFunc.name,
            parameters: parametersWithoutCancellation,
            hasReturnValue: cancellationFunc.returnType != nil
        )

        let cleanModifiers: DeclModifierListSyntax
        if let originalModifiers = cancellationFunc.modifiers, !originalModifiers.isEmpty {
            var modifiersArray: [DeclModifierSyntax] = []

            for (index, modifier) in originalModifiers.enumerated() {
                var cleanModifier = modifier

                if modifier.name.text == "open" {
                    cleanModifier = cleanModifier.with(\.name, .keyword(.public))
                }

                cleanModifier = cleanModifier.with(\.leadingTrivia, index == 0 ? .spaces(4) : .space)
                    .with(\.trailingTrivia, .space)

                modifiersArray.append(cleanModifier)
            }

            cleanModifiers = DeclModifierListSyntax(modifiersArray)
        } else {
            cleanModifiers = DeclModifierListSyntax([])
        }

        let funcKeywordLeadingTrivia: Trivia = cleanModifiers.isEmpty ? .spaces(4) : .init()

        return FunctionDeclSyntax(
            attributes: cancellationFunc.attributes ?? AttributeListSyntax([]),
            modifiers: cleanModifiers,
            funcKeyword: .keyword(.func, leadingTrivia: funcKeywordLeadingTrivia, trailingTrivia: .space),
            name: .identifier(name),
            signature: signature,
            body: functionCallBody
        )
    }

    private func createFunctionCallBody(
        cancellationFunctionName: String,
        parameters: FunctionParameterListSyntax,
        hasReturnValue: Bool
    ) -> CodeBlockSyntax {
        // Build parameter arguments for the function call
        let parameterArguments = parameters.map { parameter in
            let paramName = parameter.firstName.text
            return "\(paramName): \(paramName)"
        }.joined(separator: ", ")

        let functionCallArgs = parameterArguments.isEmpty ?
            "cancellationToken: token" :
            "\(parameterArguments), cancellationToken: token"

        return CodeBlockSyntax {
            DeclSyntax(
                """
                let token = CancellationToken()
                return try await withTaskCancellationHandler {
                    try await \(raw: cancellationFunctionName)(\(raw: functionCallArgs))
                } onCancel: {
                    do {
                        try token.cancel()
                    } catch {
                        NSLog("Failed to cancel \\(#function): \\(error)")
                    }
                }
                """
            )
        }
    }
}
