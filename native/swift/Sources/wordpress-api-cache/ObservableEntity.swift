import Foundation
import WordPressAPIInternal

public protocol ObservableEntity: Sendable {
    var entityId: EntityId { get }
}

extension FullEntityAnyPostWithEditContext: ObservableEntity {}
