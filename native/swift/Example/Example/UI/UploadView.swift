import Foundation
import SwiftUI
import PhotosUI
import WordPressAPI

struct UploadView: View {
    @Environment(\.dismiss) private var dismiss

    @StateObject
    var viewModel: UploadViewModel

    @State private var selectedItems: [MediaItem] = []
    @State private var isFilePickerPresented = false

    var body: some View {
        NavigationStack {
            VStack {
                List(selectedItems, id: \.self) { item in
                    switch item {
                    case .photo:
                        HStack {
                            Image(systemName: "photo")
                            Text("Photo")
                        }
                    case .file(let url):
                        HStack {
                            Image(systemName: "doc")
                            Text(url.lastPathComponent)
                        }
                    }
                }

                if let progress = viewModel.progress {
                    ProgressView(progress)
                        .progressViewStyle(.linear)
                        .labelsHidden()
                        .padding()
                }

                if let error = viewModel.error {
                    Text(error)
                        .foregroundStyle(.red)
                        .padding()
                }
            }
            .navigationTitle("Upload Media")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Upload") {
                        Task {
                            await viewModel.startUploading(selectedItems)
                            selectedItems = []
                        }
                    }
                    .disabled(selectedItems.isEmpty || viewModel.progress != nil)
                }

                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        viewModel.progress?.cancel()
                        dismiss()
                    }
                }

                ToolbarItem(placement: .automatic) {
                    PhotosPicker(
                        selection: Binding(
                            get: {
                                selectedItems.compactMap {
                                    if case .photo(let item) = $0 {
                                        return item
                                    } else {
                                        return nil
                                    }
                                }
                            },
                            set: { newPhotos in
                                let newItems = newPhotos.map { MediaItem.photo($0) }
                                selectedItems.removeAll(where: newItems.contains(_:))
                                selectedItems.append(contentsOf: newItems)
                            }
                        ),
                        maxSelectionCount: 10,
                        matching: .any(of: [.images, .videos])
                    ) {
                        Label("Photos", systemImage: "photo.on.rectangle")
                    }
                }

                ToolbarItem(placement: .automatic) {
                    Button {
                        isFilePickerPresented = true
                    } label: {
                        Label("Files", systemImage: "folder")
                    }
                }
            }
            .fileImporter(
                isPresented: $isFilePickerPresented,
                allowedContentTypes: [.image, .video, .text],
                allowsMultipleSelection: true
            ) { result in
                switch result {
                case .success(let files):
                    let newItems = files.map { MediaItem.file($0) }
                    selectedItems.append(contentsOf: newItems)
                case .failure(let error):
                    print("Error selecting files: \(error.localizedDescription)")
                }
            }
            .frame(minHeight: 300)
        }
    }
}

enum MediaItem: Hashable {
    case photo(PhotosPickerItem)
    case file(URL)
}

struct TransferableImage: Transferable {
    let url: URL

    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(contentType: .data) { image in
            SentTransferredFile(image.url)
        } importing: { received in
            let copy = URL.temporaryDirectory.appending(path: received.file.lastPathComponent)

            if FileManager.default.fileExists(atPath: copy.path()) {
                try FileManager.default.removeItem(at: copy)
            }

            try FileManager.default.copyItem(at: received.file, to: copy)
            return Self.init(url: copy)
        }
    }
}

@MainActor
class UploadViewModel: ObservableObject {
    @Published var error: String?
    @Published var progress: Progress?

    private let loginManager: LoginManager

    init(loginManager: LoginManager) {
        self.loginManager = loginManager
    }

    func startUploading(_ items: [MediaItem]) async {
        self.error = nil
        do {
            try await upload(items)
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func upload(_ items: [MediaItem]) async throws {
        let unitForEachChild: Int64 = 100
        let progress = Progress.discreteProgress(totalUnitCount: unitForEachChild * Int64(items.count))
        self.progress = progress
        defer { self.progress = nil }

        let api = try await WordPressAPI.instance(loginManager: self.loginManager)

        for item in items {
            let child = Progress(totalUnitCount: unitForEachChild, parent: progress, pendingUnitCount: unitForEachChild)

            let file: URL
            switch item {
            case let .file(url):
                file = url
            case let .photo(photo):
                let localPath = try await withCheckedThrowingContinuation { continuation in
                    photo.loadTransferable(type: TransferableImage.self) { result in
                        do {
                            continuation.resume(returning: try result.get())
                        } catch {
                            continuation.resume(throwing: error)
                        }
                    }
                }

                guard let localPath else {
                    preconditionFailure("Unable to obtain local file path")
                }

                file = localPath.url
            }

            let isAccessingFileOutsideSandbox = file.startAccessingSecurityScopedResource()

            guard try file.checkPromisedItemIsReachable() else {
                preconditionFailure("Item is not reachable")
            }

            NSLog("Uploading \(item)")
            _ = try await api.uploadMedia(
                params: .init(filePath: file.path),
                fulfilling: child
            )

            defer {
                if isAccessingFileOutsideSandbox {
                    file.stopAccessingSecurityScopedResource()
                }
            }

            if case .photo = item {
                try? FileManager.default.removeItem(at: file)
            }

            NSLog("Upload completed")
        }
    }
}
