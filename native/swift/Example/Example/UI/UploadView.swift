import Foundation
import SwiftUI
import PhotosUI
import WordPressAPI

struct UploadView: View {
    @Environment(\.dismiss) private var dismiss
    @StateObject private var viewModel = UploadViewModel()

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
                            get: { selectedItems.compactMap { if case .photo(let item) = $0 { return item } else { return nil } } },
                            set: { newPhotos in
                                let newItems = newPhotos.map { MediaItem.photo($0) }
                                selectedItems.append(contentsOf: newItems)
                            }
                        ),
                        maxSelectionCount: 10,
                        matching: .images
                    ) {
                        Label("Photos", systemImage: "photo.on.rectangle")
                    }
                }

                ToolbarItem(placement: .automatic) {
                    Button(action: {
                        isFilePickerPresented = true
                    }) {
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

@MainActor
private class UploadViewModel: ObservableObject {
    @Published var error: String?
    @Published var progress: Progress?

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

        let api = try await WordPressAPI.globalInstance

        for item in items {
            let child = Progress(totalUnitCount: unitForEachChild, parent: progress, pendingUnitCount: unitForEachChild)

            let file: URL
            switch item {
            case let .file(url):
                file = url
            case let .photo(photo):
                file = FileManager.default.temporaryDirectory.appendingPathComponent("photo_\(UUID().uuidString).jpg")

                NSLog("Exporting photo to \(file)")
                let data = try await photo.loadTransferable(type: Data.self)!
                try data.write(to: file)
            }

            NSLog("Uploading \(item)")
            _ = try await api.uploadMedia(
                params: .init(),
                fromLocalFileURL: file,
                fulfilling: child
            )

            if case .photo = item {
                try? FileManager.default.removeItem(at: file)
            }

            NSLog("Upload completed")
        }
    }
}
