import Foundation

import SwiftSyntax
import SwiftParser
import SwiftSyntaxBuilder

extension DeclModifierListSyntax {
    var isPublic: Bool {
        contains { $0.name.text == "public" }
    }
}

extension String {
    func toCamelCase() -> String {
        // TODO: Turn typename into camel case properly

        if isEmpty {
            return ""
        }

        return self[startIndex].lowercased() + self[index(after: startIndex)...]
    }

    func escapeReservedKeyword() -> String {
        // TODO: Implement this properly

        let isKeyword = self == "default"
        return isKeyword ? "`\(self)`" : self
    }
}

extension FunctionDeclSyntax {
    func call() -> FunctionCallExprSyntax {
        FunctionCallExprSyntax(
            calledExpression: DeclReferenceExprSyntax(baseName: .identifier(self.name.text)),
            leftParen: TokenSyntax(.leftParen, presence: .present),
            rightParen: TokenSyntax(.rightParen, presence: .present)
        ) {
            LabeledExprListSyntax {
                for parameter in self.signature.parameterClause.parameters {
                    let name = (parameter.secondName ?? parameter.firstName).text
                    LabeledExprSyntax(
                        label: parameter.firstName.kind == .wildcardPattern ? nil : parameter.firstName,
                        colon: TokenSyntax(.colon, presence: .present),
                        expression: DeclReferenceExprSyntax(baseName: .identifier(name))
                    )
                }
            }
        }
    }

    var isGlobalFunction: Bool {
        var node: any SyntaxProtocol = self
        while true {
            guard let parent = node.parent else {
                return true
            }

            if parent.isProtocol((any DeclSyntaxProtocol).self) {
                return false
            }

            node = parent
        }
    }
}

class Visitor: SyntaxVisitor {
    var structs: [StructDeclSyntax] = []
    var globalFunctions: [FunctionDeclSyntax] = []

    init() {
        super.init(viewMode: .sourceAccurate)
    }

    override func visit(_ node: StructDeclSyntax) -> SyntaxVisitorContinueKind {
        if node.modifiers.isPublic {
            structs.append(node)
        }
        return .skipChildren
    }

    override func visit(_ node: ClassDeclSyntax) -> SyntaxVisitorContinueKind {
        return .skipChildren
    }

    override func visit(_ node: EnumDeclSyntax) -> SyntaxVisitorContinueKind {
        return .skipChildren
    }

    override func visit(_ node: ExtensionDeclSyntax) -> SyntaxVisitorContinueKind {
        return .skipChildren
    }

    override func visit(_ node: ProtocolDeclSyntax) -> SyntaxVisitorContinueKind {
        return .skipChildren
    }

    override func visit(_ node: FunctionDeclSyntax) -> SyntaxVisitorContinueKind {
        if node.isGlobalFunction, node.modifiers.isPublic {
            globalFunctions.append(node)
        }

        return .skipChildren
    }
}

func createStaticFunctions(type: StructDeclSyntax, candidates: inout [FunctionDeclSyntax]) -> CodeBlockItemSyntax? {
    print("Creating static functions for struct \(type.name.text)")

    var functions = [FunctionDeclSyntax]()
    var untouched = [FunctionDeclSyntax]()
    for function in candidates {
        guard function.signature.returnClause?.type.as(IdentifierTypeSyntax.self)?.name.text == type.name.text else {
            untouched.append(function)
            continue
        }

        let camelCase = type.name.text.toCamelCase()
        let funcName = function.name.text
        guard funcName.hasPrefix(camelCase) == true else {
            untouched.append(function)
            continue
        }

        let newFuncName = String(funcName[funcName.index(funcName.startIndex, offsetBy: camelCase.count)...]).toCamelCase().escapeReservedKeyword()

        var staticFunc = function
        staticFunc.name = TokenSyntax(.identifier(newFuncName), presence: .present)
        staticFunc.modifiers = [DeclModifierSyntax(name: .keyword(.static))]
        staticFunc.body = CodeBlockSyntax {
            function.call()
        }

        functions.append(staticFunc)

        print("  - [+] \(function.name)")
    }

    candidates = untouched

    guard !functions.isEmpty else {
        return nil
    }

    let ext = ExtensionDeclSyntax(modifiers: [.init(name: .keyword(.public))], extendedType: IdentifierTypeSyntax(name: type.name)) {
        MemberBlockItemListSyntax {
            for f in functions {
                MemberBlockItemSyntax(decl: f)
            }
        }
    }

    return CodeBlockItemSyntax(item: .decl(DeclSyntax(ext)))
}

let wrapperDir = URL(string: "../wordpress-api-wrapper/", relativeTo: URL(fileURLWithPath: #filePath))!
let rustBinding = try String(contentsOf: URL(string: "./wp_api.swift", relativeTo: wrapperDir)!)
let source = Parser.parse(source: rustBinding)

let visitor = Visitor()
visitor.walk(source)

var result = SourceFileSyntax(statements: CodeBlockItemListSyntax([]))
var publicFunctions = visitor.globalFunctions
for type in visitor.structs {
    if let newCode = createStaticFunctions(type: type, candidates: &publicFunctions) {
        result.statements.append(newCode)
    }
}

let dest = URL(string: "../wordpress-api/generated.swift", relativeTo: wrapperDir)!
let fileHandle = try FileHandle(forWritingTo: dest)
try fileHandle.truncate(atOffset: 0)

fileHandle.write(
     """
    // DO NOT MODIFY: This file is auto-generated.

    import Foundation

    #if canImport(WordPressAPIInternal)
    import WordPressAPIInternal
    #endif


    """.data(using: .utf8)!
)

fileHandle.write(result.formatted().description.data(using: .utf8)!)

try fileHandle.close()
